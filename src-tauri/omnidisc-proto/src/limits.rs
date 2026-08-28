use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Limits {
    pub max_message_chars: u32,
    pub max_upload_bytes: u64,
    pub max_attachments: u8,
    pub max_guilds_per_user: u32,
    pub max_channels_per_guild: u32,
    pub max_roles_per_guild: u32,
    pub max_group_dm_members: u8,
    pub max_custom_emoji: u32,
    pub max_bio_chars: u32,
    pub max_reactions_per_message: u8,
    pub max_pins_per_channel: u32,
    pub max_voice_bitrate_kbps: u32,
    pub max_avatar_bytes: u64,
    pub max_banner_bytes: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_message_chars: 4_000,
            max_upload_bytes: 100 * 1024 * 1024,
            max_attachments: 10,
            max_guilds_per_user: 200,
            max_channels_per_guild: 500,
            max_roles_per_guild: 250,
            max_group_dm_members: 10,
            max_custom_emoji: 250,
            max_bio_chars: 320,
            max_reactions_per_message: 30,
            max_pins_per_channel: 250,
            max_voice_bitrate_kbps: 384,
            max_avatar_bytes: 8 * 1024 * 1024,
            max_banner_bytes: 16 * 1024 * 1024,
        }
    }
}
