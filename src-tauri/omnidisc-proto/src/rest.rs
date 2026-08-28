use crate::channel::{ChannelType, OverwriteTarget};
use crate::message::{Message, MessageReference};
use crate::user::User;
use crate::{Permissions, Snowflake};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_after_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invite_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: User,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateUserRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bio: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pronouns: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent_color: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGuildRequest {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateChannelRequest {
    pub name: String,
    #[serde(rename = "type", default = "default_channel_type")]
    pub kind: ChannelType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Snowflake>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

fn default_channel_type() -> ChannelType {
    ChannelType::GuildText
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateChannelRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nsfw: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slowmode_seconds: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Snowflake>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MessageQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<Snowflake>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<Snowflake>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub around: Option<Snowflake>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMessageRequest {
    #[serde(default)]
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<MessageReference>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachment_ids: Vec<Snowflake>,
    #[serde(default)]
    pub flags: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditMessageRequest {
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckRequest {
    pub message_id: Snowflake,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInviteRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<Snowflake>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<Snowflake>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_age_seconds: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_uses: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDmRequest {
    pub recipient_ids: Vec<Snowflake>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
    #[serde(default)]
    pub permissions: Permissions,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<u32>,
    #[serde(default)]
    pub hoist: bool,
    #[serde(default)]
    pub mentionable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateRoleRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Permissions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hoist: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mentionable: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PutOverwriteRequest {
    pub target_kind: OverwriteTarget,
    pub allow: Permissions,
    pub deny: Permissions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceTokenRequest {
    pub channel_id: Snowflake,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub ok: bool,
    pub version: String,
    pub uptime_seconds: u64,
}

pub mod double_option {
    use serde::{Deserialize, Deserializer};

    pub fn deserialize<'de, T, D>(d: D) -> Result<Option<Option<T>>, D::Error>
    where
        T: Deserialize<'de>,
        D: Deserializer<'de>,
    {
        Option::<T>::deserialize(d).map(Some)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchHas {
    File,
    Image,
    Link,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchQuery {
    #[serde(default)]
    pub q: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<Snowflake>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#in: Option<Snowflake>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has: Option<SearchHas>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<Snowflake>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<Snowflake>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub messages: Vec<Message>,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LimitQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkDeleteRequest {
    pub message_ids: Vec<Snowflake>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BanRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KickRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateMemberRequest {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "double_option::deserialize"
    )]
    pub nick: Option<Option<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "double_option::deserialize"
    )]
    pub muted_until: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateGuildRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "double_option::deserialize"
    )]
    pub description: Option<Option<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "double_option::deserialize"
    )]
    pub afk_channel_id: Option<Option<Snowflake>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "double_option::deserialize"
    )]
    pub system_channel_id: Option<Option<Snowflake>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferGuildRequest {
    pub user_id: Snowflake,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuditLogQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<Snowflake>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_id: Option<Snowflake>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreateRelationshipRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<Snowflake>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PutNoteRequest {
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserNote {
    pub user_id: Snowflake,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdminInstance {
    pub name: String,
    pub registration_open: bool,
    pub streaming: crate::bitrate::StreamingPolicy,
    pub limits: crate::limits::Limits,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateInstanceRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registration_open: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub streaming: Option<crate::bitrate::StreamingPolicy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limits: Option<crate::limits::Limits>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdminStats {
    pub users: u64,
    pub guilds: u64,
    pub channels: u64,
    pub messages: u64,
    pub gateway_sessions: u64,
    pub uptime_seconds: u64,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterDeviceRequest {
    pub ed25519_pub: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Device {
    pub user_id: Snowflake,
    pub device_id: String,
    pub ed25519_pub: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub created_at: String,
    pub last_seen_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyPackageUpload {
    pub ciphersuite: u16,
    pub blob: String,
    #[serde(default)]
    pub last_resort: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PutKeyPackagesRequest {
    pub key_packages: Vec<KeyPackageUpload>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyPackageCount {
    pub unclaimed: u64,
    pub last_resort: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimedKeyPackage {
    pub device_id: String,
    pub ciphersuite: u16,
    pub blob: String,
    #[serde(default)]
    pub last_resort: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimedKeyPackages {
    pub user_id: Snowflake,
    pub key_packages: Vec<ClaimedKeyPackage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMlsGroupRequest {
    pub channel_id: Snowflake,
    pub group_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MlsGroupMember {
    pub user_id: Snowflake,
    pub device_id: String,
    pub joined_epoch: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MlsGroup {
    pub group_id: String,
    pub channel_id: Snowflake,
    pub creator_device_id: String,
    pub current_epoch: u64,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<MlsGroupMember>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlsCommitRequest {
    pub epoch: u64,
    pub commit: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub welcome: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub added_devices: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed_devices: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MlsCommitResponse {
    pub group_id: String,
    pub epoch: u64,
    pub commit_envelope_id: Snowflake,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub welcome_envelope_id: Option<Snowflake>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlsMessageRequest {
    pub epoch: u64,
    pub blob: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MlsInboxQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<Snowflake>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlsInboxResponse {
    pub envelopes: Vec<crate::gateway::MlsEnvelope>,
    #[serde(default)]
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlsAckRequest {
    pub envelope_ids: Vec<Snowflake>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThumbQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub w: Option<u32>,
    #[serde(default)]
    pub exp: u64,
    #[serde(default)]
    pub sig: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SignedMediaQuery {
    #[serde(default)]
    pub exp: u64,
    #[serde(default)]
    pub sig: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mls_commit_request_defaults() {
        let r: MlsCommitRequest = serde_json::from_str(r#"{"epoch":1,"commit":"AA=="}"#).unwrap();
        assert!(r.welcome.is_none());
        assert!(r.added_devices.is_empty());
        let c: KeyPackageUpload =
            serde_json::from_str(r#"{"ciphersuite":1,"blob":"AA=="}"#).unwrap();
        assert!(!c.last_resort);
    }

    #[test]
    fn double_option_distinguishes_null_from_absent() {
        let absent: UpdateMemberRequest = serde_json::from_str("{}").unwrap();
        assert!(absent.muted_until.is_none());
        let cleared: UpdateMemberRequest = serde_json::from_str(r#"{"muted_until":null}"#).unwrap();
        assert_eq!(cleared.muted_until, Some(None));
        let set: UpdateMemberRequest =
            serde_json::from_str(r#"{"muted_until":"2030-01-01T00:00:00Z"}"#).unwrap();
        assert_eq!(
            set.muted_until.as_ref().and_then(|m| m.as_deref()),
            Some("2030-01-01T00:00:00Z")
        );
    }

    #[test]
    fn search_query_parses_in_keyword() {
        let q: SearchQuery =
            serde_json::from_str(r#"{"q":"hello","in":"42","has":"link"}"#).unwrap();
        assert_eq!(q.r#in, Some(Snowflake(42)));
        assert_eq!(q.has, Some(SearchHas::Link));
    }
}
