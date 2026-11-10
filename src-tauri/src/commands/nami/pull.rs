#![allow(clippy::all)]
#![allow(warnings)]

use super::dispatcher::{self, DownloadResult};
use super::{
    nami_load_app_settings, nami_load_profiles, omniget_nami_dir, update_profile_last_pulled,
    Profile,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct PullResult {
    pub success: bool,
    pub profile_url: String,
    pub platform: String,
    pub username: Option<String>,
    pub files_downloaded: u32,
    pub message: String,
    pub early_exited: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PullBatchProgress {
    pub current_index: usize,
    pub total: usize,
    pub profile_url: String,
    pub platform: String,
    pub username: Option<String>,
    pub files_downloaded: u32,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PullBatchSummary {
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub total_downloaded: u32,
    pub results: Vec<PullResult>,
}

pub async fn pull_single_creator(
    app: AppHandle,
    profile_url: String,
    platform: String,
    content_type: Option<String>,
    early_exit_threshold: Option<u32>,
    download_id: Option<String>,
) -> Result<PullResult, String> {
    let settings = nami_load_app_settings()?;
    let output_dir = settings.output_directory.ok_or_else(|| {
        "Please configure an output directory in Nami Settings before pulling creator updates"
            .to_string()
    })?;

    let threshold = early_exit_threshold.unwrap_or(5);
    let ctype = content_type.unwrap_or_else(|| "all".to_string());
    let id = download_id.unwrap_or_else(|| {
        format!(
            "pull_{}_{}",
            Utc::now().timestamp_millis(),
            Utc::now().timestamp_nanos_opt().unwrap_or(0) % 1_000_000
        )
    });

    let app_dir = omniget_nami_dir()?;
    let username = dispatcher::extract_username_generic(&profile_url);

    let dl_result = dispatcher::execute_download_workflow(
        app.clone(),
        profile_url.clone(),
        output_dir,
        settings.cookies_file,
        ctype,
        id,
        app_dir,
        Some(true), // skip replies
        Some(threshold),
    )
    .await?;

    let now = Utc::now().timestamp();
    if dl_result.success {
        let _ = update_profile_last_pulled(&platform, &profile_url, now);
    }

    Ok(PullResult {
        success: dl_result.success,
        profile_url,
        platform,
        username,
        files_downloaded: dl_result.files_count,
        message: dl_result.message,
        early_exited: true,
    })
}

pub async fn pull_all_creators(
    app: AppHandle,
    platform: Option<String>,
    content_type: Option<String>,
    early_exit_threshold: Option<u32>,
) -> Result<PullBatchSummary, String> {
    let target_platforms = if let Some(p) = platform {
        vec![p]
    } else {
        vec![
            "instagram".to_string(),
            "tiktok".to_string(),
            "facebook".to_string(),
            "x".to_string(),
        ]
    };

    let mut all_profiles: Vec<Profile> = Vec::new();
    for p in target_platforms {
        if let Ok(profs) = nami_load_profiles(p) {
            all_profiles.extend(profs);
        }
    }

    let total = all_profiles.len();
    let mut results: Vec<PullResult> = Vec::with_capacity(total);
    let mut succeeded = 0usize;
    let mut failed = 0usize;
    let mut total_downloaded = 0u32;

    for (idx, prof) in all_profiles.into_iter().enumerate() {
        let _ = app.emit(
            "nami_pull_batch_progress",
            PullBatchProgress {
                current_index: idx + 1,
                total,
                profile_url: prof.url.clone(),
                platform: prof.platform.clone(),
                username: prof.username.clone(),
                files_downloaded: total_downloaded,
                status: "syncing".to_string(),
            },
        );

        match pull_single_creator(
            app.clone(),
            prof.url.clone(),
            prof.platform.clone(),
            content_type.clone(),
            early_exit_threshold,
            None,
        )
        .await
        {
            Ok(res) => {
                if res.success {
                    succeeded += 1;
                } else {
                    failed += 1;
                }
                total_downloaded += res.files_downloaded;
                results.push(res);
            }
            Err(e) => {
                failed += 1;
                results.push(PullResult {
                    success: false,
                    profile_url: prof.url,
                    platform: prof.platform,
                    username: prof.username,
                    files_downloaded: 0,
                    message: e,
                    early_exited: false,
                });
            }
        }

        if idx + 1 < total {
            let jitter_ms = 1500 + (Utc::now().timestamp_millis() % 1000) as u64;
            tokio::time::sleep(Duration::from_millis(jitter_ms)).await;
        }
    }

    Ok(PullBatchSummary {
        total,
        succeeded,
        failed,
        total_downloaded,
        results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pull_result_structure() {
        let res = PullResult {
            success: true,
            profile_url: "https://instagram.com/creator".to_string(),
            platform: "instagram".to_string(),
            username: Some("creator".to_string()),
            files_downloaded: 3,
            message: "Successfully downloaded 3 files".to_string(),
            early_exited: true,
        };

        assert!(res.success);
        assert_eq!(res.files_downloaded, 3);
        assert!(res.early_exited);
    }

    #[test]
    fn test_pull_batch_summary_aggregation() {
        let items = vec![
            PullResult {
                success: true,
                profile_url: "https://instagram.com/c1".to_string(),
                platform: "instagram".to_string(),
                username: Some("c1".to_string()),
                files_downloaded: 4,
                message: "OK".to_string(),
                early_exited: true,
            },
            PullResult {
                success: false,
                profile_url: "https://tiktok.com/@c2".to_string(),
                platform: "tiktok".to_string(),
                username: Some("c2".to_string()),
                files_downloaded: 0,
                message: "Network failure".to_string(),
                early_exited: false,
            },
        ];

        let summary = PullBatchSummary {
            total: items.len(),
            succeeded: items.iter().filter(|i| i.success).count(),
            failed: items.iter().filter(|i| !i.success).count(),
            total_downloaded: items.iter().map(|i| i.files_downloaded).sum(),
            results: items,
        };

        assert_eq!(summary.total, 2);
        assert_eq!(summary.succeeded, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.total_downloaded, 4);
    }
}
