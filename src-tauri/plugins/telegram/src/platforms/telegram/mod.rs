pub mod api;
pub mod auth;
pub mod downloader;
pub mod parallel_download;
pub mod thumbnail_cache;

pub struct SubtleValidator;

impl SubtleValidator {
    pub fn validate_safe_id(id: &str) -> Result<(), String> {
        if id.is_empty()
            || id.contains("..")
            || id
                .chars()
                .any(|c| !c.is_ascii_alphanumeric() && !matches!(c, '_' | '-'))
        {
            return Err("Invalid account identifier".to_string());
        }
        Ok(())
    }
}
