pub use crate::platforms::magnet::MagnetDownloader;
pub use libreget_core::platforms::traits;
pub use libreget_core::platforms::BlueskyDownloader;
pub use libreget_core::platforms::DirectFileDownloader;
pub use libreget_core::platforms::DouyinDownloader;
pub use libreget_core::platforms::GenericYtdlpDownloader;
pub use libreget_core::platforms::InstagramDownloader;
pub use libreget_core::platforms::P2pDownloader;
pub use libreget_core::platforms::PinterestDownloader;
pub use libreget_core::platforms::Platform;
pub use libreget_core::platforms::RedditDownloader;
pub use libreget_core::platforms::ThreadsDownloader;
pub use libreget_core::platforms::TikTokDownloader;
pub use libreget_core::platforms::TwitchClipsDownloader;
pub use libreget_core::platforms::TwitterDownloader;
pub use libreget_core::platforms::VimeoDownloader;
pub use libreget_core::platforms::YouTubeDownloader;

pub mod magnet;
pub mod noop;
pub mod twitter;

#[cfg(not(target_os = "android"))]
pub mod bilibili;
#[cfg(not(target_os = "android"))]
pub mod gallerydl;
#[cfg(not(target_os = "android"))]
pub mod generic_ytdlp;
// Ported to libreget-core: bluesky, direct_file, douyin, instagram, pinterest, p2p, reddit, tiktok, twitch, twitter, vimeo, youtube
