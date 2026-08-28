use crate::channel::{Channel, Guild, Member, Role};
use crate::message::{Message, ReadState};
use crate::user::{PresenceUpdate, Relationship, User, VoiceState};
use crate::Snowflake;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

pub const HEARTBEAT_INTERVAL_MS: u64 = 30_000;
pub const IDENTIFY_TIMEOUT_MS: u64 = 10_000;
pub const MAX_FRAME_BYTES: usize = 10 * 1024 * 1024;
pub const LARGE_GUILD_THRESHOLD: u32 = 250;
pub const MEMBER_LIST_WINDOW: u32 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum Opcode {
    Dispatch = 0,
    Heartbeat = 1,
    Identify = 2,
    PresenceUpdate = 3,
    VoiceStateUpdate = 4,
    Resume = 6,
    Reconnect = 7,
    RequestGuildMembers = 8,
    InvalidSession = 9,
    Hello = 10,
    HeartbeatAck = 11,
    GatewayError = 12,
    LazyRequest = 14,
    Typing = 20,
    MlsEnvelope = 30,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize_repr, Deserialize_repr)]
#[repr(u16)]
pub enum CloseCode {
    UnknownError = 4000,
    UnknownOpcode = 4001,
    DecodeError = 4002,
    NotAuthenticated = 4003,
    AuthenticationFailed = 4004,
    AlreadyAuthenticated = 4005,
    InvalidSeq = 4007,
    RateLimited = 4008,
    SessionTimedOut = 4009,
    InvalidVersion = 4012,
    ServerRestarting = 4013,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    pub op: Opcode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub s: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub t: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub d: Option<serde_json::Value>,
}

impl Frame {
    pub fn new(op: Opcode, d: impl Serialize) -> Self {
        Self {
            op,
            s: None,
            t: None,
            d: serde_json::to_value(d).ok(),
        }
    }

    pub fn dispatch(seq: u64, event: &DispatchEvent) -> Self {
        let d = serde_json::to_value(event).ok().and_then(|v| match v {
            serde_json::Value::Object(mut m) => m.remove("d"),
            other => Some(other),
        });
        Self {
            op: Opcode::Dispatch,
            s: Some(seq),
            t: Some(event.name().to_string()),
            d,
        }
    }

    pub fn event(&self) -> Option<DispatchEvent> {
        if self.op != Opcode::Dispatch {
            return None;
        }
        let mut obj = serde_json::Map::new();
        obj.insert("t".into(), serde_json::Value::String(self.t.clone()?));
        if let Some(d) = &self.d {
            obj.insert("d".into(), d.clone());
        }
        serde_json::from_value(serde_json::Value::Object(obj)).ok()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hello {
    pub heartbeat_interval: u64,
    pub protocol_version: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identify {
    pub token: String,
    pub protocol_version: u16,
    #[serde(default)]
    pub compress: Compression,
    #[serde(default)]
    pub properties: ClientProperties,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presence: Option<PresenceUpdate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Compression {
    #[default]
    None,
    Zstd,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClientProperties {
    #[serde(default)]
    pub os: String,
    #[serde(default)]
    pub client: String,
    #[serde(default)]
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resume {
    pub token: String,
    pub session_id: String,
    pub seq: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceStateUpdateRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<Snowflake>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<Snowflake>,
    #[serde(default)]
    pub self_mute: bool,
    #[serde(default)]
    pub self_deaf: bool,
    #[serde(default)]
    pub self_stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LazyRequest {
    pub guild_id: Snowflake,
    pub channel_id: Snowflake,
    pub ranges: Vec<(u32, u32)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypingRequest {
    pub channel_id: Snowflake,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MlsEnvelope {
    pub id: Snowflake,
    pub group_id: String,
    pub epoch: u64,
    pub sender_device_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recipient_device_ids: Vec<String>,
    pub kind: MlsEnvelopeKind,
    pub payload: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MlsEnvelopeKind {
    Welcome,
    Commit,
    Proposal,
    Application,
}

impl MlsEnvelopeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Welcome => "welcome",
            Self::Commit => "commit",
            Self::Proposal => "proposal",
            Self::Application => "application",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "welcome" => Self::Welcome,
            "commit" => Self::Commit,
            "proposal" => Self::Proposal,
            "application" => Self::Application,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ready {
    pub session_id: String,
    pub user: User,
    pub guilds: Vec<Guild>,
    pub private_channels: Vec<Channel>,
    pub relationships: Vec<Relationship>,
    pub read_states: Vec<ReadState>,
    pub voice_states: Vec<VoiceState>,
    pub presences: Vec<PresenceUpdate>,
    pub instance: InstanceInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceInfo {
    pub name: String,
    pub version: String,
    pub media_url: String,
    pub sfu_url: String,
    pub max_upload_bytes: u64,
    pub streaming: crate::bitrate::StreamingPolicy,
    #[serde(default)]
    pub limits: crate::limits::Limits,
    #[serde(default)]
    pub registration_open: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceServerUpdate {
    pub guild_id: Option<Snowflake>,
    pub channel_id: Snowflake,
    pub endpoint: String,
    pub token: String,
    pub room: String,
    #[serde(default)]
    pub ice_servers: Vec<IceServer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IceServer {
    pub urls: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypingStart {
    pub channel_id: Snowflake,
    pub user_id: Snowflake,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<Snowflake>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberListUpdate {
    pub guild_id: Snowflake,
    pub channel_id: Snowflake,
    pub total: u32,
    pub online: u32,
    pub ops: Vec<MemberListOp>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum MemberListOp {
    Sync {
        range: (u32, u32),
        items: Vec<MemberListItem>,
    },
    Insert {
        index: u32,
        item: MemberListItem,
    },
    Update {
        index: u32,
        item: MemberListItem,
    },
    Delete {
        index: u32,
    },
    Invalidate {
        range: (u32, u32),
    },
}

// Wire type shared with omnidisc-server: boxing a variant would keep the JSON
// identical but break every construction site in that repo.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MemberListItem {
    Group {
        id: String,
        count: u32,
    },
    Member {
        member: Member,
        user: User,
        presence: Option<PresenceUpdate>,
    },
}

// Wire type shared with omnidisc-server: boxing a variant would keep the JSON
// identical but break every construction site in that repo.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t", content = "d")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DispatchEvent {
    Ready(Ready),
    Resumed,
    GuildCreate(Guild),
    GuildUpdate(Guild),
    GuildDelete {
        id: Snowflake,
    },
    GuildMemberAdd(Member),
    GuildMemberUpdate(Member),
    GuildMemberRemove {
        guild_id: Snowflake,
        user_id: Snowflake,
    },
    GuildMemberListUpdate(MemberListUpdate),
    GuildRoleCreate(Role),
    GuildRoleUpdate(Role),
    GuildRoleDelete {
        guild_id: Snowflake,
        id: Snowflake,
    },
    ChannelCreate(Channel),
    ChannelUpdate(Channel),
    ChannelDelete {
        id: Snowflake,
    },
    ChannelPinsUpdate {
        channel_id: Snowflake,
    },
    MessageCreate(Message),
    MessageUpdate(Message),
    MessageDelete {
        id: Snowflake,
        channel_id: Snowflake,
    },
    MessageDeleteBulk {
        ids: Vec<Snowflake>,
        channel_id: Snowflake,
    },
    MessageReactionAdd {
        channel_id: Snowflake,
        message_id: Snowflake,
        user_id: Snowflake,
        emoji: crate::message::Emoji,
    },
    MessageReactionRemove {
        channel_id: Snowflake,
        message_id: Snowflake,
        user_id: Snowflake,
        emoji: crate::message::Emoji,
    },
    MessageAck(ReadState),
    TypingStart(TypingStart),
    PresenceUpdate(PresenceUpdate),
    PresenceUpdateBulk(Vec<PresenceUpdate>),
    UserUpdate(User),
    RelationshipAdd(Relationship),
    RelationshipRemove {
        user_id: Snowflake,
    },
    VoiceStateUpdate(VoiceState),
    VoiceServerUpdate(VoiceServerUpdate),
    CallRing {
        channel_id: Snowflake,
        from_user_id: Snowflake,
    },
    MlsEnvelope(MlsEnvelope),
    DeviceRevoked {
        user_id: Snowflake,
        device_id: String,
    },
    InviteCreate(crate::channel::Invite),
    InviteDelete {
        code: String,
    },
    SessionsReplace(Vec<SessionInfo>),
}

impl DispatchEvent {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Ready(_) => "READY",
            Self::Resumed => "RESUMED",
            Self::GuildCreate(_) => "GUILD_CREATE",
            Self::GuildUpdate(_) => "GUILD_UPDATE",
            Self::GuildDelete { .. } => "GUILD_DELETE",
            Self::GuildMemberAdd(_) => "GUILD_MEMBER_ADD",
            Self::GuildMemberUpdate(_) => "GUILD_MEMBER_UPDATE",
            Self::GuildMemberRemove { .. } => "GUILD_MEMBER_REMOVE",
            Self::GuildMemberListUpdate(_) => "GUILD_MEMBER_LIST_UPDATE",
            Self::GuildRoleCreate(_) => "GUILD_ROLE_CREATE",
            Self::GuildRoleUpdate(_) => "GUILD_ROLE_UPDATE",
            Self::GuildRoleDelete { .. } => "GUILD_ROLE_DELETE",
            Self::ChannelCreate(_) => "CHANNEL_CREATE",
            Self::ChannelUpdate(_) => "CHANNEL_UPDATE",
            Self::ChannelDelete { .. } => "CHANNEL_DELETE",
            Self::ChannelPinsUpdate { .. } => "CHANNEL_PINS_UPDATE",
            Self::MessageCreate(_) => "MESSAGE_CREATE",
            Self::MessageUpdate(_) => "MESSAGE_UPDATE",
            Self::MessageDelete { .. } => "MESSAGE_DELETE",
            Self::MessageDeleteBulk { .. } => "MESSAGE_DELETE_BULK",
            Self::MessageReactionAdd { .. } => "MESSAGE_REACTION_ADD",
            Self::MessageReactionRemove { .. } => "MESSAGE_REACTION_REMOVE",
            Self::MessageAck(_) => "MESSAGE_ACK",
            Self::TypingStart(_) => "TYPING_START",
            Self::PresenceUpdate(_) => "PRESENCE_UPDATE",
            Self::PresenceUpdateBulk(_) => "PRESENCE_UPDATE_BULK",
            Self::UserUpdate(_) => "USER_UPDATE",
            Self::RelationshipAdd(_) => "RELATIONSHIP_ADD",
            Self::RelationshipRemove { .. } => "RELATIONSHIP_REMOVE",
            Self::VoiceStateUpdate(_) => "VOICE_STATE_UPDATE",
            Self::VoiceServerUpdate(_) => "VOICE_SERVER_UPDATE",
            Self::CallRing { .. } => "CALL_RING",
            Self::MlsEnvelope(_) => "MLS_ENVELOPE",
            Self::DeviceRevoked { .. } => "DEVICE_REVOKED",
            Self::InviteCreate(_) => "INVITE_CREATE",
            Self::InviteDelete { .. } => "INVITE_DELETE",
            Self::SessionsReplace(_) => "SESSIONS_REPLACE",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub client: ClientProperties,
    pub last_seen: String,
    pub current: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_frame_carries_payload_in_d() {
        let ev = DispatchEvent::ChannelDelete { id: Snowflake(42) };
        let f = Frame::dispatch(3, &ev);
        assert_eq!(f.t.as_deref(), Some("CHANNEL_DELETE"));
        assert_eq!(f.s, Some(3));
        assert_eq!(
            f.d.as_ref()
                .and_then(|d| d.get("id"))
                .and_then(|v| v.as_str()),
            Some("42")
        );
        assert!(
            matches!(f.event(), Some(DispatchEvent::ChannelDelete { id }) if id == Snowflake(42))
        );
    }

    #[test]
    fn mls_envelope_dispatch_roundtrips() {
        let env = MlsEnvelope {
            id: Snowflake(7),
            group_id: "g1".into(),
            epoch: 3,
            sender_device_id: "dev-a".into(),
            recipient_device_ids: vec!["dev-b".into()],
            kind: MlsEnvelopeKind::Commit,
            payload: "AAEC".into(),
        };
        let f = Frame::dispatch(9, &DispatchEvent::MlsEnvelope(env.clone()));
        assert_eq!(f.t.as_deref(), Some("MLS_ENVELOPE"));
        assert_eq!(
            f.d.as_ref()
                .and_then(|d| d.get("kind"))
                .and_then(|v| v.as_str()),
            Some("commit")
        );
        assert!(matches!(f.event(), Some(DispatchEvent::MlsEnvelope(e)) if e == env));
        let f = Frame::dispatch(
            10,
            &DispatchEvent::DeviceRevoked {
                user_id: Snowflake(1),
                device_id: "dev-a".into(),
            },
        );
        assert_eq!(f.t.as_deref(), Some("DEVICE_REVOKED"));
        assert_eq!(
            MlsEnvelopeKind::parse("welcome"),
            Some(MlsEnvelopeKind::Welcome)
        );
        assert_eq!(MlsEnvelopeKind::Application.as_str(), "application");
    }

    #[test]
    fn unit_event_roundtrips() {
        let f = Frame::dispatch(1, &DispatchEvent::Resumed);
        assert!(f.d.is_none());
        assert!(matches!(f.event(), Some(DispatchEvent::Resumed)));
        let json = serde_json::to_string(&f).unwrap();
        let back: Frame = serde_json::from_str(&json).unwrap();
        assert!(matches!(back.event(), Some(DispatchEvent::Resumed)));
    }
}
