use crate::{Permissions, Snowflake};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum ChannelType {
    GuildText = 0,
    Dm = 1,
    GuildVoice = 2,
    GroupDm = 3,
    GuildCategory = 4,
    GuildLink = 5,
    PersonalNotes = 6,
}

impl ChannelType {
    pub fn is_text_capable(self) -> bool {
        matches!(
            self,
            Self::GuildText | Self::Dm | Self::GroupDm | Self::GuildVoice | Self::PersonalNotes
        )
    }

    pub fn is_guild(self) -> bool {
        matches!(
            self,
            Self::GuildText | Self::GuildVoice | Self::GuildCategory | Self::GuildLink
        )
    }

    pub fn is_voice(self) -> bool {
        matches!(self, Self::GuildVoice)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelOverwrite {
    pub target_id: Snowflake,
    pub target_kind: OverwriteTarget,
    pub allow: Permissions,
    pub deny: Permissions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverwriteTarget {
    Role,
    Member,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Channel {
    pub id: Snowflake,
    #[serde(rename = "type")]
    pub kind: ChannelType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<Snowflake>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Snowflake>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
    #[serde(default)]
    pub position: i32,
    #[serde(default)]
    pub nsfw: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slowmode_seconds: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bitrate: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_limit: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub overwrites: Vec<ChannelOverwrite>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recipient_ids: Vec<Snowflake>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<Snowflake>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_message_id: Option<Snowflake>,
    #[serde(default)]
    pub e2ee: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Guild {
    pub id: Snowflake,
    pub name: String,
    pub owner_id: Snowflake,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub roles: Vec<Role>,
    #[serde(default)]
    pub channels: Vec<Channel>,
    #[serde(default)]
    pub member_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub afk_channel_id: Option<Snowflake>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_channel_id: Option<Snowflake>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Role {
    pub id: Snowflake,
    pub guild_id: Snowflake,
    pub name: String,
    pub permissions: Permissions,
    pub position: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<u32>,
    #[serde(default)]
    pub hoist: bool,
    #[serde(default)]
    pub mentionable: bool,
    #[serde(default)]
    pub is_everyone: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Member {
    pub guild_id: Snowflake,
    pub user_id: Snowflake,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nick: Option<String>,
    #[serde(default)]
    pub role_ids: Vec<Snowflake>,
    pub joined_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub muted_until: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Invite {
    pub code: String,
    pub guild_id: Option<Snowflake>,
    pub channel_id: Option<Snowflake>,
    pub inviter_id: Snowflake,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_uses: Option<u32>,
    #[serde(default)]
    pub uses: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ban {
    pub guild_id: Snowflake,
    pub user_id: Snowflake,
    pub banned_by: Snowflake,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub id: Snowflake,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<Snowflake>,
    pub actor_id: Snowflake,
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_id: Option<Snowflake>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default)]
    pub changes: serde_json::Value,
    pub created_at: String,
}
