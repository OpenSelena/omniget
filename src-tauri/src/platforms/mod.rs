pub use omniget_core::platforms::traits;
pub use omniget_core::platforms::Platform;

pub mod noop;
pub mod bluesky;
pub mod pinterest;
pub mod tiktok;
pub mod twitch;
pub mod twitter;

#[cfg(not(target_os = "android"))]
pub mod bilibili;
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
#[cfg(not(target_os = "android"))]
pub mod youtube;
