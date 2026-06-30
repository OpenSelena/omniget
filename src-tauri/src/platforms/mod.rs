pub use omniget_core::platforms::traits;
pub use omniget_core::platforms::Platform;
pub use omniget_core::platforms::GenericYtdlpDownloader;
pub use omniget_core::platforms::YouTubeDownloader;

pub mod bluesky;
pub mod direct_file;
pub mod noop;
pub mod pinterest;
pub mod tiktok;
pub mod twitch;
pub mod twitter;

#[cfg(not(target_os = "android"))]
pub mod bilibili;
#[cfg(not(target_os = "android"))]
pub mod douyin;
#[cfg(not(target_os = "android"))]
pub mod gallerydl;
#[cfg(not(target_os = "android"))]
pub mod generic_ytdlp;
#[cfg(not(target_os = "android"))]
pub mod instagram;
pub mod magnet;
pub mod p2p;
#[cfg(not(target_os = "android"))]
pub mod reddit;
#[cfg(not(target_os = "android"))]
pub mod vimeo;
// YouTube now lives in omniget-core
// #[cfg(not(target_os = "android"))]
// pub mod youtube;
