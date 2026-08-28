use crate::Snowflake;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

pub const MAX_ATTACHMENTS: usize = 10;
pub const MAX_EMBEDS: usize = 10;
pub const MAX_DISTINCT_REACTIONS: usize = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum MessageType {
    Default = 0,
    Reply = 1,
    Forward = 2,
    MemberJoin = 10,
    MemberLeave = 11,
    ChannelPinned = 12,
    CallStarted = 13,
    CallEnded = 14,
    GroupNameChanged = 15,
    GroupIconChanged = 16,
    GroupRecipientAdded = 17,
    GroupRecipientRemoved = 18,
    E2eeKeyRotated = 30,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub id: Snowflake,
    pub channel_id: Snowflake,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<Snowflake>,
    pub author_id: Snowflake,
    #[serde(rename = "type")]
    pub kind: MessageType,
    #[serde(default)]
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edited_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<MessageReference>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<Attachment>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub embeds: Vec<Embed>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reactions: Vec<ReactionCount>,
    #[serde(default)]
    pub mentions: Mentions,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub flags: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub e2ee: Option<EncryptedPayload>,
}

pub mod flags {
    pub const SUPPRESS_EMBEDS: u32 = 1 << 0;
    pub const SUPPRESS_NOTIFICATIONS: u32 = 1 << 1;
    pub const VOICE_MESSAGE: u32 = 1 << 2;
    pub const EPHEMERAL: u32 = 1 << 3;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageReference {
    pub message_id: Snowflake,
    pub channel_id: Snowflake,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<Snowflake>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<Box<Message>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Mentions {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub users: Vec<Snowflake>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<Snowflake>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub channels: Vec<Snowflake>,
    #[serde(default)]
    pub everyone: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attachment {
    pub id: Snowflake,
    pub filename: String,
    pub size: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbhash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub encrypted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Embed {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub site_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<EmbedMedia>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video: Option<EmbedMedia>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbedMedia {
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReactionCount {
    pub emoji: Emoji,
    pub count: u32,
    #[serde(default)]
    pub me: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Emoji {
    Unicode { name: String },
    Custom { id: Snowflake, name: String, animated: bool },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedPayload {
    pub group_id: String,
    pub epoch: u64,
    pub ciphertext: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadState {
    pub channel_id: Snowflake,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_read_id: Option<Snowflake>,
    #[serde(default)]
    pub mention_count: u32,
}
