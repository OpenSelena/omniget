use serde::Serialize;

use omniget_core::core::tools::yt_search::{self, SearchHit, SearchOptions};

/// Teto da tela de busca; a CLI pede mais direto ao core.
const UI_MAX_RESULTS: u32 = 20;

#[derive(Clone, Serialize)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub author: String,
    pub duration: Option<f64>,
    pub thumbnail_url: Option<String>,
    pub url: String,
    pub platform: String,
}

impl From<SearchHit> for SearchResult {
    fn from(h: SearchHit) -> Self {
        Self {
            id: h.id,
            title: h.title,
            author: h.uploader,
            duration: h.duration,
            thumbnail_url: h.thumbnail_url,
            url: h.url,
            platform: h.platform,
        }
    }
}

#[tauri::command]
pub async fn search_videos(
    query: String,
    platform: String,
    max_results: u32,
) -> Result<Vec<SearchResult>, String> {
    let opts = SearchOptions {
        query,
        platform,
        max_results: max_results.clamp(1, UI_MAX_RESULTS),
        ..Default::default()
    };
    yt_search::search(&opts)
        .await
        .map(|hits| hits.into_iter().map(SearchResult::from).collect())
        .map_err(|e| e.to_string())
}
