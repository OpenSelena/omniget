use crate::provider::Provider;
use crate::{MlsError, Result};
use data_encoding::BASE32_NOPAD;
use hkdf::Hkdf;
use openmls::prelude::*;
use openmls_basic_credential::SignatureKeyPair;
use openmls_traits::OpenMlsProvider;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tls_codec::{Deserialize as TlsDeserialize, Serialize as TlsSerialize};

const CIPHERSUITE: Ciphersuite = Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;

/// Numeric ciphersuite the server stores alongside every key package.
pub const CIPHERSUITE_ID: u16 = 0x0001;
pub const VOICE_EXPORTER_LABEL: &str = "omnidisc-voice";
pub const VOICE_EXPORTER_INFO: &[u8] = b"omnidisc-voice-v1 livekit-shared-key";

/// Out-of-band verification string: base32 of SHA-256(pubkey)[..20], in groups
/// of four, e.g. `EYES-OGGZ-JXJN-6ENS-3VW3-KVPN-NHDO-SJ3V`.
pub fn fingerprint(public_key: &[u8]) -> String {
    let digest = Sha256::digest(public_key);
    let encoded = BASE32_NOPAD.encode(&digest[..20]);
    encoded
        .as_bytes()
        .chunks(4)
        .map(|c| std::str::from_utf8(c).unwrap_or("????"))
        .collect::<Vec<_>>()
        .join("-")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberInfo {
    pub user_id: String,
    pub device_id: String,
    pub fingerprint: String,
    pub is_me: bool,
}

/// A device the caller has confirmed out of the MLS band: the identity it was
/// asked about and the signature key that device published on the server.
/// Everything the server hands us is checked against one of these, because a
/// hostile server can otherwise swap in a device of its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceRef {
    pub user_id: String,
    pub device_id: String,
    pub signature_key: Vec<u8>,
}

impl DeviceRef {
    pub fn new(user_id: impl Into<String>, device_id: impl Into<String>, key: Vec<u8>) -> Self {
        Self { user_id: user_id.into(), device_id: device_id.into(), signature_key: key }
    }

    fn matches(&self, user_id: &str, device_id: &str, key: &[u8]) -> bool {
        self.user_id == user_id && self.device_id == device_id && self.signature_key == key
    }
}

/// A key package claimed from the server, bound to the device it must belong to.
#[derive(Debug, Clone)]
pub struct ClaimedDevice {
    pub device: DeviceRef,
    pub key_package: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct CommitOutput {
    pub commit: Vec<u8>,
    pub welcome: Option<Vec<u8>>,
    /// Epoch the group reaches once the server accepts this commit.
    pub epoch: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Incoming {
    Application {
        user_id: String,
        device_id: String,
        /// The signature key of the leaf that sent this. MLS authenticates the
        /// leaf, never the identity string inside it, so the caller still has
        /// to match this against the device roster it fetched itself.
        signature_key: Vec<u8>,
        plaintext: Vec<u8>,
    },
    Commit { epoch: u64, removed_me: bool },
    Proposal,
}

pub struct MlsClient {
    user_id: String,
    device_id: String,
    provider: Provider,
    signer: SignatureKeyPair,
    credential: CredentialWithKey,
    public_key: [u8; 32],
    groups: HashMap<String, MlsGroup>,
}

fn mls<E: std::fmt::Debug>(what: &str) -> impl FnOnce(E) -> MlsError + '_ {
    move |e| {
        let text = format!("{e:?}");
        if text.contains("UseAfterEviction") {
            MlsError::Evicted
        } else {
            MlsError::Mls(format!("{what}: {text}"))
        }
    }
}

fn identity_parts(credential: &Credential) -> (String, String) {
    let raw = BasicCredential::try_from(credential.clone())
        .map(|b| String::from_utf8_lossy(b.identity()).into_owned())
        .unwrap_or_default();
    match raw.split_once(':') {
        Some((user, device)) => (user.to_string(), device.to_string()),
        None => (raw, String::new()),
    }
}

impl MlsClient {
    pub fn new(user_id: &str, device_id: &str, seed: &[u8; 32]) -> Result<Self> {
        Self::build(user_id, device_id, seed, Provider::fresh(), false, &[])
    }

    /// Rebuild from a blob produced by [`MlsClient::export_state`].
    pub fn restore(user_id: &str, device_id: &str, seed: &[u8; 32], blob: &[u8]) -> Result<Self> {
        let (group_ids, storage) = split_state(blob)?;
        let provider = Provider::from_blob(storage)?;
        Self::build(user_id, device_id, seed, provider, true, &group_ids)
    }

    fn build(
        user_id: &str,
        device_id: &str,
        seed: &[u8; 32],
        provider: Provider,
        restored: bool,
        group_ids: &[String],
    ) -> Result<Self> {
        let signing = ed25519_dalek::SigningKey::from_bytes(seed);
        let public_key = signing.verifying_key().to_bytes();
        let signer = if restored {
            SignatureKeyPair::read(provider.storage(), &public_key, SignatureScheme::ED25519)
                .ok_or_else(|| MlsError::State("device key missing from stored state".into()))?
        } else {
            let pair = SignatureKeyPair::from_raw(
                SignatureScheme::ED25519,
                signing.to_bytes().to_vec(),
                public_key.to_vec(),
            );
            pair.store(provider.storage()).map_err(mls("store signer"))?;
            pair
        };
        let credential = CredentialWithKey {
            credential: BasicCredential::new(format!("{user_id}:{device_id}").into_bytes()).into(),
            signature_key: public_key.to_vec().into(),
        };
        let mut groups = HashMap::new();
        for id in group_ids {
            let gid = GroupId::from_slice(id.as_bytes());
            match MlsGroup::load(provider.storage(), &gid).map_err(mls("load group"))? {
                Some(group) => {
                    groups.insert(id.clone(), group);
                }
                // A group listed in the snapshot but missing from storage means a
                // partial write; dropping it is safer than refusing to start.
                None => {}
            }
        }
        Ok(Self {
            user_id: user_id.to_string(),
            device_id: device_id.to_string(),
            provider,
            signer,
            credential,
            public_key,
            groups,
        })
    }

    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    pub fn public_key(&self) -> [u8; 32] {
        self.public_key
    }

    pub fn fingerprint(&self) -> String {
        fingerprint(&self.public_key)
    }

    pub fn export_state(&self) -> Vec<u8> {
        let mut ids: Vec<&String> = self.groups.keys().collect();
        ids.sort();
        let mut out = Vec::new();
        out.extend_from_slice(&(ids.len() as u32).to_be_bytes());
        for id in ids {
            let bytes = id.as_bytes();
            out.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
            out.extend_from_slice(bytes);
        }
        out.extend_from_slice(&self.provider.to_blob());
        out
    }

    pub fn group_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.groups.keys().cloned().collect();
        ids.sort();
        ids
    }

    pub fn has_group(&self, group_id: &str) -> bool {
        self.groups.contains_key(group_id)
    }

    pub fn epoch(&self, group_id: &str) -> Option<u64> {
        self.groups.get(group_id).map(|g| g.epoch().as_u64())
    }

    /// Fresh key packages to publish. The last one is marked `last_resort` when
    /// asked, so a claim never fails just because the stock ran out.
    pub fn key_packages(&mut self, count: usize, last_resort: bool) -> Result<Vec<Vec<u8>>> {
        let mut out = Vec::with_capacity(count + usize::from(last_resort));
        for _ in 0..count {
            out.push(self.build_key_package(false)?);
        }
        if last_resort {
            out.push(self.build_key_package(true)?);
        }
        Ok(out)
    }

    fn build_key_package(&mut self, last_resort: bool) -> Result<Vec<u8>> {
        let builder = KeyPackage::builder();
        let builder = if last_resort { builder.mark_as_last_resort() } else { builder };
        let bundle = builder
            .build(CIPHERSUITE, &self.provider, &self.signer, self.credential.clone())
            .map_err(mls("key package"))?;
        let out: MlsMessageOut = bundle.into();
        out.tls_serialize_detached().map_err(mls("serialise key package"))
    }

    pub fn create_group(&mut self, group_id: &str) -> Result<()> {
        if self.groups.contains_key(group_id) {
            return Ok(());
        }
        let config = MlsGroupCreateConfig::builder()
            .ciphersuite(CIPHERSUITE)
            .use_ratchet_tree_extension(true)
            .build();
        let group = MlsGroup::new_with_group_id(
            &self.provider,
            &self.signer,
            &config,
            GroupId::from_slice(group_id.as_bytes()),
            self.credential.clone(),
        )
        .map_err(mls("create group"))?;
        self.groups.insert(group_id.to_string(), group);
        Ok(())
    }

    /// Parse a claimed key package and bind it to the device it must belong to.
    /// `validate` only proves the package is self-consistent; without the two
    /// checks below the server could hand us a package of its own and an honest
    /// client would add the server's device to the group.
    fn parse_key_package(&self, bytes: &[u8], expected: &DeviceRef) -> Result<KeyPackage> {
        let msg = MlsMessageIn::tls_deserialize(&mut &bytes[..])
            .map_err(|e| MlsError::Malformed(format!("key package: {e}")))?;
        let kp = match msg.extract() {
            MlsMessageBodyIn::KeyPackage(kp) => kp
                .validate(self.provider.crypto(), ProtocolVersion::Mls10)
                .map_err(mls("validate key package"))?,
            _ => return Err(MlsError::Malformed("not a key package".into())),
        };
        let leaf = kp.leaf_node();
        let (user_id, device_id) = identity_parts(leaf.credential());
        if user_id != expected.user_id || device_id != expected.device_id {
            return Err(MlsError::Untrusted(format!(
                "a key package claimed for {}:{} carries the identity {}:{}",
                expected.user_id, expected.device_id, user_id, device_id
            )));
        }
        if leaf.signature_key().as_slice() != expected.signature_key.as_slice() {
            return Err(MlsError::Untrusted(format!(
                "the key package for {}:{} is signed by a key that device never published",
                expected.user_id, expected.device_id
            )));
        }
        Ok(kp)
    }

    /// Stage a commit adding devices. The commit is NOT merged: the caller must
    /// call [`MlsClient::merge_pending`] only after the server accepted it, and
    /// [`MlsClient::clear_pending`] on a 409 — merging a commit the server
    /// rejected would fork the group for this device.
    pub fn add_members(&mut self, group_id: &str, devices: &[ClaimedDevice]) -> Result<CommitOutput> {
        let parsed = devices
            .iter()
            .map(|d| self.parse_key_package(&d.key_package, &d.device))
            .collect::<Result<Vec<_>>>()?;
        let Self { groups, provider, signer, .. } = self;
        let group = groups
            .get_mut(group_id)
            .ok_or_else(|| MlsError::UnknownGroup(group_id.to_string()))?;
        let (commit, welcome, _info) = group
            .add_members(provider, signer, &parsed)
            .map_err(mls("add_members"))?;
        Ok(CommitOutput {
            commit: commit.tls_serialize_detached().map_err(mls("serialise commit"))?,
            welcome: Some(welcome.tls_serialize_detached().map_err(mls("serialise welcome"))?),
            epoch: group.epoch().as_u64() + 1,
        })
    }

    pub fn remove_devices(&mut self, group_id: &str, device_ids: &[String]) -> Result<CommitOutput> {
        let leaves: Vec<LeafNodeIndex> = {
            let group = self
                .groups
                .get(group_id)
                .ok_or_else(|| MlsError::UnknownGroup(group_id.to_string()))?;
            group
                .members()
                .filter(|m| {
                    let (_, device) = identity_parts(&m.credential);
                    device_ids.iter().any(|d| *d == device)
                })
                .map(|m| m.index)
                .collect()
        };
        if leaves.is_empty() {
            return Err(MlsError::Malformed("none of those devices are in the group".into()));
        }
        let Self { groups, provider, signer, .. } = self;
        let group = groups
            .get_mut(group_id)
            .ok_or_else(|| MlsError::UnknownGroup(group_id.to_string()))?;
        let (commit, welcome, _info) = group
            .remove_members(provider, signer, &leaves)
            .map_err(mls("remove_members"))?;
        Ok(CommitOutput {
            commit: commit.tls_serialize_detached().map_err(mls("serialise commit"))?,
            welcome: welcome
                .map(|w| w.tls_serialize_detached().map_err(mls("serialise welcome")))
                .transpose()?,
            epoch: group.epoch().as_u64() + 1,
        })
    }

    pub fn merge_pending(&mut self, group_id: &str) -> Result<u64> {
        let Self { groups, provider, .. } = self;
        let group = groups
            .get_mut(group_id)
            .ok_or_else(|| MlsError::UnknownGroup(group_id.to_string()))?;
        group.merge_pending_commit(provider).map_err(mls("merge_pending_commit"))?;
        Ok(group.epoch().as_u64())
    }

    pub fn clear_pending(&mut self, group_id: &str) -> Result<()> {
        let Self { groups, provider, .. } = self;
        let group = groups
            .get_mut(group_id)
            .ok_or_else(|| MlsError::UnknownGroup(group_id.to_string()))?;
        group
            .clear_pending_commit(provider.storage())
            .map_err(mls("clear_pending_commit"))
    }

    /// Join a group we were welcomed into.
    ///
    /// A Welcome is the one MLS message that creates state out of nothing, so
    /// it is the one the server can forge a whole group with. Three refusals
    /// keep it honest: the group id has to be the one the caller expected, we
    /// must not already have that group (joining again would silently re-key
    /// the channel onto whoever sent the second Welcome), and the member that
    /// added us has to be a device the caller already knows is a recipient of
    /// that channel.
    pub fn join_welcome(
        &mut self,
        bytes: &[u8],
        expected_group_id: &str,
        allowed_senders: &[DeviceRef],
    ) -> Result<String> {
        if self.groups.contains_key(expected_group_id) {
            return Err(MlsError::GroupExists(expected_group_id.to_string()));
        }
        let msg = MlsMessageIn::tls_deserialize(&mut &bytes[..])
            .map_err(|e| MlsError::Malformed(format!("welcome: {e}")))?;
        let welcome = match msg.extract() {
            MlsMessageBodyIn::Welcome(w) => w,
            _ => return Err(MlsError::Malformed("not a welcome".into())),
        };
        let staged = StagedWelcome::new_from_welcome(
            &self.provider,
            &MlsGroupJoinConfig::default(),
            welcome,
            None,
        )
        .map_err(mls("staged welcome"))?;
        let id =
            String::from_utf8_lossy(staged.group_context().group_id().as_slice()).into_owned();
        if id != expected_group_id {
            return Err(MlsError::Untrusted(format!(
                "a Welcome announced as {expected_group_id} is really for {id}"
            )));
        }
        if self.groups.contains_key(&id) {
            return Err(MlsError::GroupExists(id));
        }
        let sender = staged.welcome_sender().map_err(mls("welcome sender"))?;
        let (sender_user, sender_device) = identity_parts(sender.credential());
        let sender_key = sender.signature_key().as_slice().to_vec();
        if !allowed_senders
            .iter()
            .any(|d| d.matches(&sender_user, &sender_device, &sender_key))
        {
            return Err(MlsError::Untrusted(format!(
                "{sender_user}:{sender_device} is not a known device of anyone in {id}"
            )));
        }
        let group = staged.into_group(&self.provider).map_err(mls("into_group"))?;
        self.groups.insert(id.clone(), group);
        Ok(id)
    }

    pub fn encrypt(&mut self, group_id: &str, plaintext: &[u8]) -> Result<(Vec<u8>, u64)> {
        let Self { groups, provider, signer, .. } = self;
        let group = groups
            .get_mut(group_id)
            .ok_or_else(|| MlsError::UnknownGroup(group_id.to_string()))?;
        let out = group
            .create_message(provider, signer, plaintext)
            .map_err(mls("create_message"))?;
        Ok((
            out.tls_serialize_detached().map_err(mls("serialise message"))?,
            group.epoch().as_u64(),
        ))
    }

    pub fn process(&mut self, group_id: &str, bytes: &[u8]) -> Result<Incoming> {
        let msg = MlsMessageIn::tls_deserialize(&mut &bytes[..])
            .map_err(|e| MlsError::Malformed(format!("envelope: {e}")))?;
        let protocol = msg
            .try_into_protocol_message()
            .map_err(|e| MlsError::Malformed(format!("not a protocol message: {e:?}")))?;
        let Self { groups, provider, .. } = self;
        let group = groups
            .get_mut(group_id)
            .ok_or_else(|| MlsError::UnknownGroup(group_id.to_string()))?;
        let processed = group
            .process_message(provider, protocol)
            .map_err(mls("process_message"))?;
        let (user_id, device_id) = identity_parts(processed.credential());
        let signature_key = match processed.sender() {
            Sender::Member(index) => {
                let index = *index;
                group
                    .members()
                    .find(|m| m.index == index)
                    .map(|m| m.signature_key)
                    .unwrap_or_default()
            }
            _ => Vec::new(),
        };
        match processed.into_content() {
            ProcessedMessageContent::ApplicationMessage(app) => Ok(Incoming::Application {
                user_id,
                device_id,
                signature_key,
                plaintext: app.into_bytes(),
            }),
            ProcessedMessageContent::StagedCommitMessage(staged) => {
                let removed_me = staged.self_removed();
                group
                    .merge_staged_commit(provider, *staged)
                    .map_err(mls("merge_staged_commit"))?;
                Ok(Incoming::Commit { epoch: group.epoch().as_u64(), removed_me })
            }
            ProcessedMessageContent::ProposalMessage(p) => {
                group
                    .store_pending_proposal(provider.storage(), *p)
                    .map_err(mls("store_pending_proposal"))?;
                Ok(Incoming::Proposal)
            }
            ProcessedMessageContent::ExternalJoinProposalMessage(_) => Ok(Incoming::Proposal),
        }
    }

    /// A removed member's group stays around and errors on every later call, so
    /// the client has to drop it itself (S-05 gotcha 6).
    pub fn drop_group(&mut self, group_id: &str) -> Result<()> {
        if let Some(mut group) = self.groups.remove(group_id) {
            let _ = group.delete(self.provider.storage());
        }
        Ok(())
    }

    pub fn members(&self, group_id: &str) -> Vec<MemberInfo> {
        let Some(group) = self.groups.get(group_id) else { return vec![] };
        group
            .members()
            .map(|m| {
                let (user_id, device_id) = identity_parts(&m.credential);
                let is_me = user_id == self.user_id && device_id == self.device_id;
                MemberInfo {
                    user_id,
                    device_id,
                    fingerprint: fingerprint(&m.signature_key),
                    is_me,
                }
            })
            .collect()
    }

    pub fn member_device_ids(&self, group_id: &str) -> Vec<String> {
        self.members(group_id).into_iter().map(|m| m.device_id).collect()
    }

    /// Room key for LiveKit's shared-key E2EE mode. Same on every member, new on
    /// every epoch, undefined for a device that was removed.
    pub fn voice_key(&self, group_id: &str, room_id: &[u8]) -> Result<[u8; 32]> {
        let group = self
            .groups
            .get(group_id)
            .ok_or_else(|| MlsError::UnknownGroup(group_id.to_string()))?;
        let exported = group
            .export_secret(self.provider.crypto(), VOICE_EXPORTER_LABEL, room_id, 32)
            .map_err(mls("export_secret"))?;
        let hk = Hkdf::<Sha256>::new(Some(room_id), &exported);
        let mut key = [0u8; 32];
        hk.expand(VOICE_EXPORTER_INFO, &mut key)
            .map_err(|_| MlsError::Mls("hkdf expand".into()))?;
        Ok(key)
    }
}

fn split_state(blob: &[u8]) -> Result<(Vec<String>, &[u8])> {
    if blob.len() < 4 {
        return Err(MlsError::State("truncated state".into()));
    }
    let mut cursor = 0usize;
    let count = u32::from_be_bytes([blob[0], blob[1], blob[2], blob[3]]) as usize;
    cursor += 4;
    let mut ids = Vec::with_capacity(count.min(1024));
    for _ in 0..count {
        if blob.len() < cursor + 4 {
            return Err(MlsError::State("truncated group list".into()));
        }
        let len = u32::from_be_bytes([
            blob[cursor],
            blob[cursor + 1],
            blob[cursor + 2],
            blob[cursor + 3],
        ]) as usize;
        cursor += 4;
        if blob.len() < cursor + len {
            return Err(MlsError::State("truncated group id".into()));
        }
        ids.push(String::from_utf8_lossy(&blob[cursor..cursor + len]).into_owned());
        cursor += len;
    }
    Ok((ids, &blob[cursor..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    const GROUP: &str = "od-group-000000001";
    const ROOM: &[u8] = b"voice-room-1";

    fn client(user: &str, device: &str, seed: u8) -> MlsClient {
        MlsClient::new(user, device, &[seed; 32]).expect("client")
    }

    fn device_ref(c: &MlsClient) -> DeviceRef {
        DeviceRef::new(c.user_id(), c.device_id(), c.public_key().to_vec())
    }

    /// One claimed key package, correctly bound to the device that made it.
    fn claim(c: &mut MlsClient) -> ClaimedDevice {
        ClaimedDevice {
            device: device_ref(c),
            key_package: c.key_packages(1, false).expect("kp").remove(0),
        }
    }

    /// Add `joiners` to `owner`'s group and hand each of them the welcome.
    fn add_all(owner: &mut MlsClient, joiners: &mut [&mut MlsClient]) -> CommitOutput {
        let claimed: Vec<ClaimedDevice> = joiners.iter_mut().map(|c| claim(c)).collect();
        let out = owner.add_members(GROUP, &claimed).expect("add");
        owner.merge_pending(GROUP).expect("merge");
        let welcome = out.welcome.clone().expect("welcome");
        let allowed = [device_ref(owner)];
        for joiner in joiners.iter_mut() {
            let id = joiner.join_welcome(&welcome, GROUP, &allowed).expect("join");
            assert_eq!(id, GROUP);
        }
        out
    }

    #[test]
    fn three_devices_exchange_messages_and_rotate_epochs() {
        let mut alice = client("1001", "desktop-a", 1);
        let mut bob = client("2002", "phone-bbb", 2);
        let mut carol = client("3003", "tablet-cc", 3);
        alice.create_group(GROUP).expect("create");
        assert_eq!(alice.epoch(GROUP), Some(0));

        let out = add_all(&mut alice, &mut [&mut bob]);
        assert_eq!(out.epoch, 1);
        assert_eq!(alice.epoch(GROUP), Some(1));
        assert_eq!(bob.epoch(GROUP), Some(1));

        let (ct, epoch) = alice.encrypt(GROUP, b"hello bob").expect("encrypt");
        assert_eq!(epoch, 1);
        assert!(!ct.windows(9).any(|w| w == b"hello bob"), "ciphertext leaked the plaintext");
        match bob.process(GROUP, &ct).expect("decrypt") {
            Incoming::Application { user_id, device_id, plaintext, .. } => {
                assert_eq!(user_id, "1001");
                assert_eq!(device_id, "desktop-a");
                assert_eq!(plaintext, b"hello bob");
            }
            other => panic!("expected an application message, got {other:?}"),
        }

        // Carol joins: epoch advances for everyone, and the existing member has
        // to process the commit to keep up.
        let carol_kp = claim(&mut carol);
        let commit = alice.add_members(GROUP, &[carol_kp]).expect("add carol");
        alice.merge_pending(GROUP).expect("merge");
        assert!(matches!(
            bob.process(GROUP, &commit.commit).expect("bob commit"),
            Incoming::Commit { epoch: 2, removed_me: false }
        ));
        carol
            .join_welcome(&commit.welcome.clone().expect("welcome"), GROUP, &[device_ref(&alice)])
            .expect("carol join");
        assert_eq!(carol.epoch(GROUP), Some(2));
        assert_eq!(alice.members(GROUP).len(), 3);

        let (ct, _) = carol.encrypt(GROUP, b"carol here").expect("encrypt");
        for member in [&mut alice, &mut bob] {
            match member.process(GROUP, &ct).expect("decrypt") {
                Incoming::Application { plaintext, .. } => assert_eq!(plaintext, b"carol here"),
                other => panic!("expected an application message, got {other:?}"),
            }
        }
    }

    #[test]
    fn a_removed_device_cannot_read_what_comes_next() {
        let mut alice = client("1001", "desktop-a", 11);
        let mut bob = client("2002", "phone-bbb", 12);
        let mut carol = client("3003", "tablet-cc", 13);
        alice.create_group(GROUP).expect("create");
        add_all(&mut alice, &mut [&mut bob, &mut carol]);

        let out = alice.remove_devices(GROUP, &["tablet-cc".to_string()]).expect("remove");
        alice.merge_pending(GROUP).expect("merge");
        assert!(matches!(
            bob.process(GROUP, &out.commit).expect("bob"),
            Incoming::Commit { removed_me: false, .. }
        ));
        assert!(matches!(
            carol.process(GROUP, &out.commit).expect("carol"),
            Incoming::Commit { removed_me: true, .. }
        ));

        let (ct, _) = alice.encrypt(GROUP, b"after the removal").expect("encrypt");
        assert!(matches!(
            bob.process(GROUP, &ct).expect("bob decrypt"),
            Incoming::Application { .. }
        ));
        assert!(carol.process(GROUP, &ct).is_err(), "a removed device decrypted a new message");
        carol.drop_group(GROUP).expect("drop");
        assert!(!carol.has_group(GROUP));
    }

    #[test]
    fn a_rejected_commit_can_be_cleared_and_retried() {
        let mut alice = client("1001", "desktop-a", 21);
        let mut bob = client("2002", "phone-bbb", 22);
        let mut carol = client("3003", "tablet-cc", 23);
        alice.create_group(GROUP).expect("create");
        add_all(&mut alice, &mut [&mut bob]);

        // Alice stages a commit adding Carol, the server rejects it (409), and
        // Bob's commit lands instead.
        let carol_kp = claim(&mut carol);
        alice.add_members(GROUP, &[carol_kp]).expect("stage");
        alice.clear_pending(GROUP).expect("clear");
        let bob_commit = bob.remove_devices(GROUP, &["desktop-a".to_string()]);
        assert!(bob_commit.is_ok(), "bob can still commit after alice cleared hers");
    }

    #[test]
    fn voice_key_agrees_per_epoch_and_changes_with_membership() {
        let mut alice = client("1001", "desktop-a", 31);
        let mut bob = client("2002", "phone-bbb", 32);
        let mut carol = client("3003", "tablet-cc", 33);
        alice.create_group(GROUP).expect("create");
        add_all(&mut alice, &mut [&mut bob]);
        let k1 = alice.voice_key(GROUP, ROOM).expect("key");
        assert_eq!(k1, bob.voice_key(GROUP, ROOM).expect("key"));

        let carol_kp = claim(&mut carol);
        let commit = alice.add_members(GROUP, &[carol_kp]).expect("add");
        alice.merge_pending(GROUP).expect("merge");
        bob.process(GROUP, &commit.commit).expect("bob");
        carol
            .join_welcome(&commit.welcome.clone().expect("welcome"), GROUP, &[device_ref(&alice)])
            .expect("join");
        let k2 = alice.voice_key(GROUP, ROOM).expect("key");
        assert_ne!(k1, k2, "the voice key must rotate on a membership change");
        assert_eq!(k2, bob.voice_key(GROUP, ROOM).expect("key"));
        assert_eq!(k2, carol.voice_key(GROUP, ROOM).expect("key"));
        assert_ne!(k2, alice.voice_key(GROUP, b"another-room").expect("key"));
    }

    #[test]
    fn state_survives_a_restart() {
        let seed = [41u8; 32];
        let mut alice = MlsClient::new("1001", "desktop-a", &seed).expect("client");
        let mut bob = client("2002", "phone-bbb", 42);
        alice.create_group(GROUP).expect("create");
        add_all(&mut alice, &mut [&mut bob]);

        let blob = alice.export_state();
        let mut restored = MlsClient::restore("1001", "desktop-a", &seed, &blob).expect("restore");
        assert_eq!(restored.group_ids(), vec![GROUP.to_string()]);
        assert_eq!(restored.epoch(GROUP), alice.epoch(GROUP));
        assert_eq!(restored.fingerprint(), alice.fingerprint());

        let (ct, _) = restored.encrypt(GROUP, b"after the restart").expect("encrypt");
        match bob.process(GROUP, &ct).expect("decrypt") {
            Incoming::Application { plaintext, .. } => assert_eq!(plaintext, b"after the restart"),
            other => panic!("expected an application message, got {other:?}"),
        }
        let (back, _) = bob.encrypt(GROUP, b"and back").expect("encrypt");
        match restored.process(GROUP, &back).expect("decrypt") {
            Incoming::Application { plaintext, .. } => assert_eq!(plaintext, b"and back"),
            other => panic!("expected an application message, got {other:?}"),
        }
        assert!(MlsClient::restore("1001", "desktop-a", &seed, &blob[..2]).is_err());
    }

    /// C1: a hostile server fabricates a second group under the id of a channel
    /// the victim already has, welcomes them into it, and every later message
    /// goes to whoever owns that group. The second Welcome must bounce.
    #[test]
    fn a_second_welcome_for_a_group_we_already_have_is_refused() {
        let mut alice = client("1001", "desktop-a", 51);
        let mut bob = client("2002", "phone-bbb", 52);
        let mut attacker = client("6006", "evil-0001", 53);
        alice.create_group(GROUP).expect("create");
        add_all(&mut alice, &mut [&mut bob]);
        let before = bob.epoch(GROUP).expect("epoch");

        // The attacker builds their own group under the very same id.
        attacker.create_group(GROUP).expect("create");
        let bob_kp = claim(&mut bob);
        let hijack = attacker.add_members(GROUP, &[bob_kp]).expect("add");
        attacker.merge_pending(GROUP).expect("merge");
        let welcome = hijack.welcome.clone().expect("welcome");

        let err = bob
            .join_welcome(&welcome, GROUP, &[device_ref(&attacker)])
            .expect_err("the second welcome must be refused");
        assert!(matches!(err, MlsError::GroupExists(_)), "{err:?}");
        assert_eq!(bob.epoch(GROUP), Some(before), "the group was re-keyed anyway");

        let (ct, _) = alice.encrypt(GROUP, b"still ours").expect("encrypt");
        match bob.process(GROUP, &ct).expect("decrypt") {
            Incoming::Application { plaintext, .. } => assert_eq!(plaintext, b"still ours"),
            other => panic!("expected an application message, got {other:?}"),
        }
    }

    /// C1: the same attack against a channel the victim has no group for yet.
    /// Nothing local contradicts it, so the only defence is the sender: the
    /// attacker is not a recipient of that channel.
    #[test]
    fn a_welcome_from_someone_outside_the_channel_is_refused() {
        let mut alice = client("1001", "desktop-a", 61);
        let mut attacker = client("6006", "evil-0001", 62);
        let mut bob = client("2002", "phone-bbb", 63);
        attacker.create_group(GROUP).expect("create");
        let bob_kp = claim(&mut bob);
        let out = attacker.add_members(GROUP, &[bob_kp]).expect("add");
        attacker.merge_pending(GROUP).expect("merge");
        let welcome = out.welcome.clone().expect("welcome");

        // Only Alice is a recipient of this channel, so only her devices may add us.
        let allowed = [device_ref(&alice)];
        let err = bob
            .join_welcome(&welcome, GROUP, &allowed)
            .expect_err("a stranger must not be able to welcome us");
        assert!(matches!(err, MlsError::Untrusted(_)), "{err:?}");
        assert!(!bob.has_group(GROUP));

        // Staging a welcome consumes the key package it was addressed to, so the
        // group-id check gets its own victim and its own welcome.
        let mut carol = client("3003", "tablet-cc", 64);
        let carol_kp = claim(&mut carol);
        let out = attacker.add_members(GROUP, &[carol_kp]).expect("add");
        attacker.merge_pending(GROUP).expect("merge");
        let err = carol
            .join_welcome(
                &out.welcome.clone().expect("welcome"),
                "od-other-channel",
                &[device_ref(&attacker)],
            )
            .expect_err("the announced group id must match the welcome");
        assert!(matches!(err, MlsError::Untrusted(_)), "{err:?}");
        assert!(!carol.has_group("od-other-channel"));
        assert!(!carol.has_group(GROUP));
    }

    /// C2: the server hands back a key package of its own under the device id it
    /// was asked for. Binding to the published signature key is what catches it.
    #[test]
    fn a_key_package_that_is_not_the_claimed_device_is_refused() {
        let mut alice = client("1001", "desktop-a", 71);
        let mut bob = client("2002", "phone-bbb", 72);
        let mut attacker = client("6006", "evil-0001", 73);
        alice.create_group(GROUP).expect("create");

        // Right identity string, wrong key: the server minted the package itself.
        let substituted = ClaimedDevice {
            device: device_ref(&bob),
            key_package: attacker.key_packages(1, false).expect("kp").remove(0),
        };
        let err = alice
            .add_members(GROUP, &[substituted])
            .expect_err("a substituted key package must be refused");
        assert!(matches!(err, MlsError::Untrusted(_)), "{err:?}");

        // Right key, but claimed for a device id we never asked about.
        let mut renamed = claim(&mut bob);
        renamed.device.device_id = "phone-zzz".into();
        let err = alice
            .add_members(GROUP, &[renamed])
            .expect_err("a key package for another device must be refused");
        assert!(matches!(err, MlsError::Untrusted(_)), "{err:?}");

        // And the honest one still goes through.
        let honest = claim(&mut bob);
        alice.add_members(GROUP, &[honest]).expect("add");
        assert_eq!(alice.members(GROUP).len(), 1, "a refused commit must not be staged");
    }

    /// C2: attribution. The identity string in a leaf is whatever its owner
    /// typed, so the caller has to match the signature key against the roster.
    #[test]
    fn application_messages_carry_the_senders_signature_key() {
        let mut alice = client("1001", "desktop-a", 81);
        let mut bob = client("2002", "phone-bbb", 82);
        alice.create_group(GROUP).expect("create");
        add_all(&mut alice, &mut [&mut bob]);

        let (ct, _) = alice.encrypt(GROUP, b"from alice").expect("encrypt");
        match bob.process(GROUP, &ct).expect("decrypt") {
            Incoming::Application { user_id, device_id, signature_key, .. } => {
                assert_eq!(user_id, "1001");
                assert_eq!(device_id, "desktop-a");
                assert_eq!(signature_key, alice.public_key().to_vec());
                assert!(device_ref(&alice).matches(&user_id, &device_id, &signature_key));
                // A roster that does not list this key must not vouch for it.
                let impostor = DeviceRef::new("1001", "desktop-a", vec![0u8; 32]);
                assert!(!impostor.matches(&user_id, &device_id, &signature_key));
            }
            other => panic!("expected an application message, got {other:?}"),
        }
    }

    #[test]
    fn fingerprints_are_stable_grouped_base32() {
        let fp = fingerprint(&[0u8; 32]);
        assert_eq!(fp.len(), 32 + 7);
        assert!(fp.split('-').all(|c| c.len() == 4));
        assert_eq!(fp, fingerprint(&[0u8; 32]));
        assert_ne!(fp, fingerprint(&[1u8; 32]));
    }
}
