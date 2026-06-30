pub use omniget_core::platforms::traits;
pub use omniget_core::platforms::Platform;
pub use omniget_core::platforms::BlueskyDownloader;
pub use omniget_core::platforms::DirectFileDownloader;
pub use omniget_core::platforms::DouyinDownloader;
pub use omniget_core::platforms::GenericYtdlpDownloader;
pub use crate::platforms::magnet::MagnetDownloader;
pub use omniget_core::platforms::P2pDownloader;
pub use omniget_core::platforms::TwitchClipsDownloader;
pub use omniget_core::platforms::VimeoDownloader;
pub use omniget_core::platforms::YouTubeDownloader;

pub mod noop;
pub mod pinterest;
pub mod tiktok;
pub mod twitter;

#[cfg(not(target_os = "android"))]
pub mod bilibili;
#[cfg(not(target_os = "android"))]
pub mod gallerydl;
#[cfg(not(target_os = "android"))]
pub mod generic_ytdlp;
#[cfg(not(target_os = "android"))]
pub mod instagram;
#[cfg(not(target_os = "android"))]
pub mod reddit;
pub mod magnet;
// Ported to omniget-core: bluesky, direct_file, douyin, p2p, twitch, vimeo, youtube
