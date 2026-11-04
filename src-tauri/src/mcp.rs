//! Servidor MCP embutido (estudos 22 e 23): as tools da seção Tools expostas
//! como ferramentas MCP em `POST /mcp` no bridge local, com o mesmo bearer
//! da extensão. Transporte "Streamable HTTP" só com respostas JSON (sem SSE),
//! que é o que Claude Code, Cursor, Goose e o `mcp-remote` do Claude Desktop
//! aceitam. Sem crate de MCP: o protocolo aqui é JSON-RPC com quatro métodos.

use omniget_core::core::tools::{
    self as tools, ai_keys, disk, dupes, edge_tts, file_search, humanize, image_resize, ocr, pdf,
    pricing, ryd, sponsorblock, startup, sysclean, uninstall, whisper, x,
};
use serde::Serialize;
use serde_json::{json, Value};
use tauri::AppHandle;

pub const PROTOCOL: &str = "2025-06-18";

#[derive(Debug, Clone, Serialize)]
pub struct ToolDef {
    pub name: &'static str,
    pub description: &'static str,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

fn obj(props: Value, required: &[&str]) -> Value {
    json!({ "type": "object", "properties": props, "required": required })
}

pub fn tools() -> Vec<ToolDef> {
    let t = |name, description, input_schema| ToolDef {
        name,
        description,
        input_schema,
    };
    vec![
        t("download_url", "Queue a URL (video, audio, playlist, course, image) in the OmniGet Downloads panel. Same as pasting it in the app.", obj(json!({ "url": { "type": "string" } }), &["url"])),
        t("youtube_sponsorblock", "SponsorBlock segments (sponsor, intro, outro, selfpromo…) of a YouTube video.", obj(json!({ "url": { "type": "string" }, "categories": { "type": "array", "items": { "type": "string" } } }), &["url"])),
        t("youtube_dislikes", "Return YouTube Dislike estimates for a video.", obj(json!({ "url": { "type": "string" } }), &["url"])),
        t("pdf_info", "Pages, size, title, author and whether the PDF has a text layer.", obj(json!({ "path": { "type": "string" } }), &["path"])),
        t("pdf_merge", "Merge PDFs into one file, in the given order.", obj(json!({ "inputs": { "type": "array", "items": { "type": "string" } }, "output": { "type": "string", "description": "Inside the OmniGet download folder; relative paths start there, empty means the folder itself." } }), &["inputs"])),
        t("pdf_split", "Split a PDF: mode each | every | ranges (\"1-3; 4-10\") | extract (\"1,3,5-7\").", obj(json!({ "input": { "type": "string" }, "mode": { "type": "string" }, "every": { "type": "integer" }, "ranges": { "type": "string" }, "output_dir": { "type": "string", "description": "Inside the OmniGet download folder; relative paths start there, empty means the folder itself." } }), &["input", "mode"])),
        t("pdf_text", "Extract the text of a PDF (optionally a page range like \"1-3, 5\").", obj(json!({ "path": { "type": "string" }, "pages": { "type": "string" } }), &["path"])),
        t("pdf_render", "Render PDF pages to PNG or JPG files.", obj(json!({ "input": { "type": "string" }, "pages": { "type": "string" }, "dpi": { "type": "integer" }, "format": { "type": "string", "enum": ["png", "jpg"] }, "output_dir": { "type": "string", "description": "Inside the OmniGet download folder; relative paths start there, empty means the folder itself." } }), &["input"])),
        t("pdf_sanitize", "Rebuild a PDF from pixels (Dangerzone-style) so scripts, forms and attachments are dropped.", obj(json!({ "input": { "type": "string" }, "output_dir": { "type": "string", "description": "Inside the OmniGet download folder; relative paths start there, empty means the folder itself." } }), &["input"])),
        t("tts_speak", "Text to speech with Microsoft Edge neural voices; writes an MP3.", obj(json!({ "text": { "type": "string" }, "voice": { "type": "string", "description": "e.g. pt-BR-AntonioNeural, en-US-AriaNeural" }, "output": { "type": "string", "description": "Inside the OmniGet download folder; relative paths start there, empty means the folder itself." } }), &["text"])),
        t("transcribe", "Transcribe audio or video locally with whisper.cpp; returns text and SRT path.", obj(json!({ "input": { "type": "string" }, "model": { "type": "string", "description": "GGML model id, default base" }, "language": { "type": "string", "description": "auto | pt | en …" }, "output_dir": { "type": "string", "description": "folder inside the download folder (default: the download folder)" } }), &["input"])),
        t("image_resize", "Resize images in batch. mode width | height | fit | percent.", obj(json!({ "inputs": { "type": "array", "items": { "type": "string" } }, "mode": { "type": "string" }, "value": { "type": "integer" }, "value2": { "type": "integer" }, "format": { "type": "string" }, "output_dir": { "type": "string", "description": "Inside the OmniGet download folder; relative paths start there, empty means the folder itself." } }), &["inputs", "mode", "value"])),
        t("ocr", "Extract text from images with Tesseract.", obj(json!({ "inputs": { "type": "array", "items": { "type": "string" } }, "langs": { "type": "string", "description": "por+eng" } }), &["inputs"])),
        t("find_duplicates", "Find duplicate files (same content) under folders.", obj(json!({ "dirs": { "type": "array", "items": { "type": "string" } }, "min_size": { "type": "integer" } }), &["dirs"])),
        t("file_search", "Search files by name (Everything, Spotlight or locate/find).", obj(json!({ "query": { "type": "string" }, "folder": { "type": "string" }, "limit": { "type": "integer" } }), &["query"])),
        t("ai_prices", "Search LLM prices per million tokens (LiteLLM + models.dev).", obj(json!({ "query": { "type": "string" }, "limit": { "type": "integer" } }), &["query"])),
        t("humanize", "Rewrite AI-sounding text so it reads like a person wrote it, using the app's configured AI.", obj(json!({ "text": { "type": "string" } }), &["text"])),
        t("x_post", "Fetch an X/Twitter post (text, author, media) by URL or id.", obj(json!({ "url": { "type": "string" } }), &["url"])),
        t("x_thread", "Unroll an X/Twitter thread from any post in it.", obj(json!({ "url": { "type": "string" } }), &["url"])),
        t("x_profile", "Profile analytics for an X/Twitter user (engagement, best hours, top posts).", obj(json!({ "handle": { "type": "string" }, "limit": { "type": "integer" } }), &["handle"])),
        t("x_search", "Search X/Twitter posts (advanced operators supported).", obj(json!({ "query": { "type": "string" }, "feed": { "type": "string", "enum": ["latest", "top"] } }), &["query"])),
        t("x_trends", "Current X/Twitter trends.", obj(json!({}), &[])),
        t("instagram_profile", "Public info of an Instagram profile, using the cookies captured by the OmniGet extension.", obj(json!({ "username": { "type": "string" }, "account": { "type": "string", "description": "cookie slot, default _default" } }), &["username"])),
        t("gallery_download", "Download a gallery/profile with gallery-dl (Pinterest, ArtStation, DeviantArt, Reddit…).", obj(json!({ "url": { "type": "string" }, "dest": { "type": "string", "description": "Inside the OmniGet download folder; relative paths start there, empty means the folder itself." } }), &["url"])),
        t("aria2_download", "Download a large file with aria2 (multi-connection).", obj(json!({ "url": { "type": "string" }, "dest_dir": { "type": "string", "description": "Inside the OmniGet download folder; relative paths start there, empty means the folder itself." }, "connections": { "type": "integer" } }), &["url"])),
        t("disk_volumes", "Mounted volumes with total and free space.", obj(json!({}), &[])),
        t("disk_scan", "Folder sizes tree and largest files under a path.", obj(json!({ "path": { "type": "string" }, "depth": { "type": "integer" } }), &["path"])),
        t("clean_scan", "What the cache cleaner would remove (rule, size, files). Does not delete anything.", obj(json!({}), &[])),
        t("startup_items", "Programs that start with the system.", obj(json!({}), &[])),
        t("installed_apps", "Installed applications with version and size.", obj(json!({}), &[])),
        t("ai_keys", "Saved AI API accounts (names, providers, balances). Keys are never returned.", obj(json!({}), &[])),
        t("download_enqueue", "Queue a URL in the Downloads panel using the app defaults, and return the queue item. mode audio downloads audio only.", obj(json!({ "url": { "type": "string" }, "mode": { "type": "string", "enum": ["video", "audio"] } }), &["url"])),
        t("downloads_queue", "List the Downloads queue with per-item status, progress, speed, ETA and output path.", obj(json!({ "status": { "type": "string", "enum": ["queued", "active", "paused", "seeding", "complete", "error"] }, "limit": { "type": "integer" } }), &[])),
        t("download_status", "One download by id: status, percent, speed, ETA, file path and the last yt-dlp command.", obj(json!({ "download_id": { "type": "integer" } }), &["download_id"])),
        t("download_cancel", "Cancel a download by id (queued, active, paused or seeding).", obj(json!({ "download_id": { "type": "integer" } }), &["download_id"])),
        t("download_pause", "Pause an active download by id.", obj(json!({ "download_id": { "type": "integer" } }), &["download_id"])),
        t("download_resume", "Resume a paused download by id.", obj(json!({ "download_id": { "type": "integer" } }), &["download_id"])),
    ]
}

fn s(v: &Value, k: &str) -> String {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string()
}
fn list(v: &Value, k: &str) -> Vec<String> {
    v.get(k)
        .and_then(|x| x.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}
fn num(v: &Value, k: &str) -> Option<u64> {
    v.get(k).and_then(|x| x.as_u64())
}

/// Argumento de texto obrigatorio, com erro legivel quando falta.
fn need_str(v: &Value, k: &str) -> Result<String, String> {
    let value = s(v, k);
    if value.trim().is_empty() {
        return Err(format!("argument \"{}\" is required (string)", k));
    }
    Ok(value)
}

/// Id de download: aceita numero ou string numerica.
fn need_id(v: &Value, k: &str) -> Result<u64, String> {
    v.get(k)
        .and_then(|x| {
            x.as_u64()
                .or_else(|| x.as_str().and_then(|t| t.trim().parse().ok()))
        })
        .ok_or_else(|| format!("argument \"{}\" is required (download id, integer)", k))
}
fn to_json<T: Serialize>(v: T) -> Result<Value, String> {
    serde_json::to_value(v).map_err(|e| e.to_string())
}
fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

// ── Downloads: o engine que ja existe, exposto como tools ──────────────
//
// Nada de capacidade nova aqui: cada arm abaixo chama exatamente o que os
// comandos Tauri de `commands/downloads.rs` chamam.

fn status_key(status: &crate::core::queue::QueueStatus) -> &'static str {
    use crate::core::queue::QueueStatus as S;
    match status {
        S::Queued => "queued",
        S::Active => "active",
        S::Paused => "paused",
        S::Seeding => "seeding",
        S::Complete { .. } => "complete",
        S::Error { .. } => "error",
    }
}

pub(crate) const QUEUE_STATUS_KEYS: &[&str] =
    &["queued", "active", "paused", "seeding", "complete", "error"];

async fn queue_snapshot(app: &AppHandle) -> Vec<crate::core::queue::QueueItemInfo> {
    use tauri::Manager;
    let state = app.state::<crate::AppState>();
    let q = state.download_queue.lock().await;
    q.get_state()
}

async fn queue_item(app: &AppHandle, id: u64) -> Result<crate::core::queue::QueueItemInfo, String> {
    queue_snapshot(app)
        .await
        .into_iter()
        .find(|i| i.id == id)
        .ok_or_else(|| format!("no download with id {}", id))
}

/// Fecha a sessao do torrent quando o item cancelado era um magnet.
async fn drop_torrent(app: &AppHandle, torrent_id: Option<usize>, pause_only: bool) {
    use tauri::Manager;
    let Some(tid) = torrent_id else { return };
    let state = app.state::<crate::AppState>();
    let session = state.torrent_session.lock().await;
    let Some(session) = session.as_ref() else {
        return;
    };
    let handle = librqbit::api::TorrentIdOrHash::Id(tid);
    if pause_only {
        if let Some(h) = session.get(handle) {
            let _ = session.pause(&h).await;
        }
    } else {
        let _ = session.delete(handle, false).await;
    }
}

/// Cancelar / pausar / retomar: a mesma sequencia dos comandos Tauri
/// (mexe na fila, emite o estado novo e tenta comecar o proximo).
async fn queue_control(app: &AppHandle, id: u64, action: &str) -> Result<Value, String> {
    use tauri::Manager;
    let state = app.state::<crate::AppState>();
    let (changed, torrent_id) = {
        let mut q = state.download_queue.lock().await;
        match action {
            "cancel" => q.cancel(id),
            "pause" => {
                let ok = q.pause(id);
                let tid = q
                    .items
                    .iter()
                    .find(|i| i.id == id)
                    .and_then(|i| i.torrent_id);
                (ok, if ok { tid } else { None })
            }
            _ => (q.resume(id), None),
        }
    };
    if !changed {
        let current = queue_item(app, id)
            .await
            .map(|i| status_key(&i.status).to_string())
            .unwrap_or_else(|e| e);
        return Err(format!("cannot {} download {} ({})", action, id, current));
    }
    if action != "resume" {
        drop_torrent(app, torrent_id, action == "pause").await;
    }
    let snapshot = {
        let q = state.download_queue.lock().await;
        q.get_state()
    };
    crate::core::queue::emit_queue_state_from_state(app, snapshot);
    crate::core::queue::try_start_next(app.clone(), state.download_queue.clone()).await;
    let item = queue_item(app, id).await.ok();
    Ok(json!({ "download_id": id, "action": action, "item": item }))
}

pub async fn call(app: &AppHandle, name: &str, a: Value) -> Result<Value, String> {
    let p = tools::noop_progress();
    match name {
        "download_url" => {
            let url = s(&a, "url");
            let action = crate::external_url::handle_external_url(app, url.clone(), "mcp").await?;
            Ok(json!({ "url": url, "action": format!("{:?}", action).to_lowercase() }))
        }
        "youtube_sponsorblock" => to_json(
            sponsorblock::segments(&s(&a, "url"), &list(&a, "categories"))
                .await
                .map_err(err)?,
        ),
        "youtube_dislikes" => to_json(ryd::votes(&s(&a, "url")).await.map_err(err)?),
        "pdf_info" => {
            let path = s(&a, "path");
            to_json(
                tokio::task::spawn_blocking(move || pdf::info(&path, None))
                    .await
                    .map_err(err)?
                    .map_err(err)?,
            )
        }
        "pdf_merge" => {
            let opts = pdf::MergeOptions {
                inputs: list(&a, "inputs"),
                output: confined(app, &a, "output", Some("merged.pdf"))?,
            };
            to_json(
                tokio::task::spawn_blocking(move || pdf::merge(&opts, &p))
                    .await
                    .map_err(err)?
                    .map_err(err)?,
            )
        }
        "pdf_split" => {
            let opts = pdf::SplitOptions {
                input: s(&a, "input"),
                mode: s(&a, "mode"),
                every: num(&a, "every").unwrap_or(0) as usize,
                ranges: s(&a, "ranges"),
                output_dir: confined(app, &a, "output_dir", None)?,
            };
            to_json(
                tokio::task::spawn_blocking(move || pdf::split(&opts, &p))
                    .await
                    .map_err(err)?
                    .map_err(err)?,
            )
        }
        "pdf_text" => {
            let (path, pages) = (s(&a, "path"), s(&a, "pages"));
            to_json(
                tokio::task::spawn_blocking(move || pdf::to_text(&path, &pages, false, ""))
                    .await
                    .map_err(err)?
                    .map_err(err)?,
            )
        }
        "pdf_render" => {
            let opts = pdf::RenderOptions {
                input: s(&a, "input"),
                pages: s(&a, "pages"),
                dpi: num(&a, "dpi").unwrap_or(0) as u32,
                format: s(&a, "format"),
                quality: 0,
                output_dir: confined(app, &a, "output_dir", None)?,
            };
            to_json(
                tokio::task::spawn_blocking(move || pdf::render(&opts, &p))
                    .await
                    .map_err(err)?
                    .map_err(err)?,
            )
        }
        "pdf_sanitize" => {
            let (input, dir) = (s(&a, "input"), confined(app, &a, "output_dir", None)?);
            to_json(
                tokio::task::spawn_blocking(move || pdf::sanitize(&input, &dir, 0, 0, &p))
                    .await
                    .map_err(err)?
                    .map_err(err)?,
            )
        }
        "tts_speak" => {
            let voice = {
                let v = s(&a, "voice");
                if v.is_empty() {
                    "pt-BR-AntonioNeural".to_string()
                } else {
                    v
                }
            };
            let opts: edge_tts::TtsOptions =
                serde_json::from_value(json!({ "text": s(&a, "text"), "voice": voice }))
                    .map_err(err)?;
            to_json(
                edge_tts::synthesize(
                    opts,
                    std::path::Path::new(&confined(app, &a, "output", Some("speech.mp3"))?),
                    p,
                )
                .await
                .map_err(err)?,
            )
        }
        "transcribe" => {
            let model = {
                let m = s(&a, "model");
                if m.is_empty() {
                    "base".to_string()
                } else {
                    m
                }
            };
            let language = {
                let l = s(&a, "language");
                if l.is_empty() {
                    "auto".to_string()
                } else {
                    l
                }
            };
            let opts = transcribe_options(&a, model, language, &download_root(app))?;
            let r = whisper::transcribe(opts, p).await.map_err(err)?;
            Ok(
                json!({ "language": r.language, "text": r.text, "srt": r.srt_path, "vtt": r.vtt_path, "txt": r.txt_path, "seconds": r.seconds }),
            )
        }
        "image_resize" => {
            let opts: image_resize::ResizeOptions = serde_json::from_value(json!({ "inputs": list(&a, "inputs"), "mode": s(&a, "mode"), "value": num(&a, "value").unwrap_or(1024), "value2": num(&a, "value2").unwrap_or(0), "format": s(&a, "format"), "output_dir": confined(app, &a, "output_dir", None)? })).map_err(err)?;
            to_json(image_resize::run(opts, p).await.map_err(err)?)
        }
        "ocr" => to_json(
            ocr::run(&list(&a, "inputs"), &s(&a, "langs"), p)
                .await
                .map_err(err)?,
        ),
        "find_duplicates" => {
            let opts: dupes::DupesOptions = serde_json::from_value(json!({ "dirs": list(&a, "dirs"), "min_size": num(&a, "min_size").unwrap_or(1024) })).map_err(err)?;
            to_json(
                tokio::task::spawn_blocking(move || dupes::scan(&opts, &p))
                    .await
                    .map_err(err)?,
            )
        }
        "file_search" => to_json(
            file_search::search(
                &s(&a, "query"),
                &s(&a, "folder"),
                num(&a, "limit").unwrap_or(100) as usize,
            )
            .await
            .map_err(err)?,
        ),
        "ai_prices" => to_json(
            pricing::search(&s(&a, "query"), "", num(&a, "limit").unwrap_or(30) as usize)
                .await
                .map_err(err)?,
        ),
        "humanize" => Ok(json!({ "text": humanize::humanize(&s(&a, "text"), None).await? })),
        "x_post" => {
            let input = s(&a, "url");
            let id = x::post_id_from(&input).ok_or_else(|| format!("not an X post: {}", input))?;
            to_json(x::fx::status(&id).await.map_err(err)?)
        }
        "x_thread" => to_json(x::thread::unroll(&s(&a, "url")).await.map_err(err)?),
        "x_profile" => to_json(
            x::profile::analyze(
                &s(&a, "handle"),
                num(&a, "limit").unwrap_or(100) as usize,
                false,
            )
            .await
            .map_err(err)?,
        ),
        "x_search" => {
            let feed = {
                let f = s(&a, "feed");
                if f.is_empty() {
                    "latest".to_string()
                } else {
                    f
                }
            };
            to_json(
                x::search::search(&s(&a, "query"), &feed, None)
                    .await
                    .map_err(err)?,
            )
        }
        "x_trends" => to_json(x::search::trends().await.map_err(err)?),
        "instagram_profile" => {
            let account = {
                let v = s(&a, "account");
                if v.is_empty() {
                    None
                } else {
                    Some(v)
                }
            };
            let client = crate::commands::tools::instagram::load_client(account.as_deref())?;
            to_json(
                tools::instagram::profile::resolve_user(&client, &s(&a, "username"))
                    .await
                    .map_err(err)?,
            )
        }
        "gallery_download" => to_json(
            tools::gallery::download(&s(&a, "url"), &confined(app, &a, "dest", None)?, None, p)
                .await
                .map_err(err)?,
        ),
        "aria2_download" => {
            let opts: tools::aria2::Aria2Options = serde_json::from_value(json!({ "url": s(&a, "url"), "dest_dir": confined(app, &a, "dest_dir", None)?, "connections": num(&a, "connections").unwrap_or(16) })).map_err(err)?;
            to_json(tools::aria2::download(opts, p).await.map_err(err)?)
        }
        "disk_volumes" => to_json(disk::volumes()),
        "disk_scan" => {
            let (path, depth) = (s(&a, "path"), num(&a, "depth").unwrap_or(2) as usize);
            to_json(
                tokio::task::spawn_blocking(move || disk::scan(&path, depth, 25, &p))
                    .await
                    .map_err(err)?
                    .map_err(err)?,
            )
        }
        "clean_scan" => to_json(
            tokio::task::spawn_blocking(move || sysclean::scan(&p))
                .await
                .map_err(err)?,
        ),
        "startup_items" => to_json(startup::list().await),
        "installed_apps" => to_json(uninstall::list(p).await),
        "ai_keys" => to_json(ai_keys::list()),
        "download_enqueue" => {
            let url = need_str(&a, "url")?;
            let mode = match s(&a, "mode").as_str() {
                "" | "video" => None,
                "audio" => Some("audio".to_string()),
                other => {
                    return Err(format!(
                        "argument \"mode\" must be \"video\" or \"audio\", got \"{}\"",
                        other
                    ))
                }
            };
            let before: std::collections::HashSet<u64> =
                queue_snapshot(app).await.iter().map(|i| i.id).collect();
            let outcome =
                crate::external_url::queue_url_with_defaults(app, url.clone(), false, mode).await?;
            let after = queue_snapshot(app).await;
            let item = after
                .iter()
                .find(|i| !before.contains(&i.id) && i.url == url)
                .or_else(|| after.iter().find(|i| i.url == url));
            let outcome = match outcome {
                crate::external_url::QueueUrlOutcome::Queued => "queued",
                crate::external_url::QueueUrlOutcome::AlreadyQueued => "already-queued",
            };
            Ok(json!({ "url": url, "outcome": outcome, "item": item }))
        }
        "downloads_queue" => {
            let want = s(&a, "status").trim().to_lowercase();
            if !want.is_empty() && !QUEUE_STATUS_KEYS.contains(&want.as_str()) {
                return Err(format!(
                    "argument \"status\" must be one of {}, got \"{}\"",
                    QUEUE_STATUS_KEYS.join(", "),
                    want
                ));
            }
            let limit = num(&a, "limit").unwrap_or(50).max(1) as usize;
            let items = queue_snapshot(app).await;
            let mut by_status: std::collections::BTreeMap<&str, u64> = Default::default();
            for i in &items {
                *by_status.entry(status_key(&i.status)).or_insert(0) += 1;
            }
            let total = items.len();
            let matched: Vec<_> = items
                .into_iter()
                .filter(|i| want.is_empty() || status_key(&i.status) == want)
                .collect();
            let shown = matched.len().min(limit);
            Ok(
                json!({ "total": total, "matched": matched.len(), "shown": shown, "by_status": by_status, "items": matched.into_iter().take(limit).collect::<Vec<_>>() }),
            )
        }
        "download_status" => {
            let id = need_id(&a, "download_id")?;
            let item = queue_item(app, id).await?;
            let command = omniget_core::core::ytdlp::get_command(id);
            let log = crate::core::download_log::get(id);
            let log: Vec<String> = log.into_iter().rev().take(20).rev().collect();
            to_json(json!({ "item": item, "command": command, "log": log }))
        }
        "download_cancel" => queue_control(app, need_id(&a, "download_id")?, "cancel").await,
        "download_pause" => queue_control(app, need_id(&a, "download_id")?, "pause").await,
        "download_resume" => queue_control(app, need_id(&a, "download_id")?, "resume").await,
        _ => Err(format!("unknown tool: {}", name)),
    }
}

// ── Estado (ligado/desligado) ──────────────────────────────────────────

fn config_file() -> Option<std::path::PathBuf> {
    tools::tools_dir().map(|d| d.join("mcp.json"))
}

fn read_config() -> serde_json::Map<String, Value> {
    config_file()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

fn write_config(cfg: &serde_json::Map<String, Value>) -> Result<(), String> {
    let p = config_file().ok_or("no data dir")?;
    std::fs::create_dir_all(p.parent().unwrap()).map_err(err)?;
    std::fs::write(&p, serde_json::to_string_pretty(cfg).unwrap()).map_err(err)
}

pub fn enabled() -> bool {
    read_config()
        .get("enabled")
        .and_then(|b| b.as_bool())
        .unwrap_or(false)
}

pub fn set_enabled(on: bool) -> Result<(), String> {
    let mut cfg = read_config();
    cfg.insert("enabled".into(), json!(on));
    write_config(&cfg)
}

/// Bearer token for MCP clients. It is separate from the browser extension
/// token: a leaked extension token must not unlock the MCP tools, which can
/// read and write files. Created on first use and kept in `mcp.json`.
pub fn token() -> Result<String, String> {
    let mut cfg = read_config();
    if let Some(t) = cfg.get("token").and_then(|t| t.as_str()) {
        if !t.trim().is_empty() {
            return Ok(t.to_string());
        }
    }
    let fresh = crate::local_bridge::generate_token();
    cfg.insert("token".into(), json!(fresh));
    write_config(&cfg)?;
    Ok(fresh)
}

/// Replaces the MCP token; every configured client must be updated.
pub fn rotate_token() -> Result<String, String> {
    let mut cfg = read_config();
    let fresh = crate::local_bridge::generate_token();
    cfg.insert("token".into(), json!(fresh));
    write_config(&cfg)?;
    Ok(fresh)
}

// ── Write confinement ──────────────────────────────────────────────────

/// Resolves a path an MCP client wants a tool to write to and keeps it inside
/// `root` (the download folder). Empty means `root` itself (or
/// `root/default_name` when a file name is needed); relative paths are taken
/// from `root`. `..` is refused, and symlinks are resolved on the part of the
/// path that already exists, so a link cannot lead outside either.
pub fn confine_write_path(
    raw: &str,
    root: &std::path::Path,
    default_name: Option<&str>,
) -> Result<std::path::PathBuf, String> {
    use std::path::{Component, PathBuf};
    if root.as_os_str().is_empty() {
        return Err("set a download folder in OmniGet settings first".into());
    }
    std::fs::create_dir_all(root).map_err(err)?;
    let root = std::fs::canonicalize(root).map_err(err)?;
    let raw = raw.trim();
    let wanted: PathBuf = if raw.is_empty() {
        match default_name {
            Some(name) => root.join(name),
            None => root.clone(),
        }
    } else {
        let p = PathBuf::from(raw);
        if p.is_absolute() {
            p
        } else {
            root.join(p)
        }
    };
    if wanted
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        return Err(format!("path must not contain \"..\": {}", raw));
    }
    // Canonicalize the deepest part that exists, then re-append the rest.
    let mut existing = wanted.clone();
    let mut rest: Vec<std::ffi::OsString> = Vec::new();
    while !existing.exists() {
        match (existing.file_name(), existing.parent()) {
            (Some(name), Some(parent)) => {
                rest.push(name.to_os_string());
                existing = parent.to_path_buf();
            }
            _ => break,
        }
    }
    let mut resolved = std::fs::canonicalize(&existing).map_err(err)?;
    for part in rest.into_iter().rev() {
        resolved.push(part);
    }
    if !resolved.starts_with(&root) {
        return Err(format!(
            "MCP tools can only write inside the download folder ({}); refused: {}",
            root.display(),
            raw
        ));
    }
    Ok(resolved)
}

/// Subtitles from the MCP `transcribe` tool go to the download folder (or a
/// folder inside it), never next to an arbitrary input file.
fn transcribe_options(
    a: &Value,
    model: String,
    language: String,
    root: &std::path::Path,
) -> Result<whisper::TranscribeOptions, String> {
    let output_dir = confine_write_path(&s(a, "output_dir"), root, None)?;
    serde_json::from_value(json!({
        "input": s(a, "input"),
        "model": model,
        "language": language,
        "output_dir": output_dir.to_string_lossy(),
    }))
    .map_err(err)
}

fn download_root(app: &AppHandle) -> std::path::PathBuf {
    crate::storage::config::load_settings(app)
        .download
        .default_output_dir
}

fn confined(
    app: &AppHandle,
    a: &Value,
    key: &str,
    default_name: Option<&str>,
) -> Result<String, String> {
    confine_write_path(&s(a, key), &download_root(app), default_name)
        .map(|p| p.to_string_lossy().to_string())
}

// ── JSON-RPC ───────────────────────────────────────────────────────────

fn rpc_error(id: Value, code: i64, message: impl Into<String>) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message.into() } })
}

fn rpc_ok(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

/// Trata uma mensagem. `None` = notificação (sem resposta).
pub async fn handle(app: &AppHandle, msg: &Value) -> Option<Value> {
    let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let id = msg.get("id").cloned();
    let params = msg.get("params").cloned().unwrap_or(Value::Null);
    if method.starts_with("notifications/") {
        return None;
    }
    let id = id?;
    Some(match method {
        "initialize" => {
            let requested = params
                .get("protocolVersion")
                .and_then(|v| v.as_str())
                .unwrap_or(PROTOCOL);
            let version = if matches!(requested, "2024-11-05" | "2025-03-26" | "2025-06-18") {
                requested
            } else {
                PROTOCOL
            };
            rpc_ok(
                id,
                json!({
                    "protocolVersion": version,
                    "capabilities": { "tools": { "listChanged": false } },
                    "serverInfo": { "name": "OmniGet", "version": env!("CARGO_PKG_VERSION") },
                    "instructions": "OmniGet desktop tools: downloads, PDF, speech, images, files, X/Twitter, Instagram, system. Paths are local to this machine."
                }),
            )
        }
        "ping" => rpc_ok(id, json!({})),
        "tools/list" => rpc_ok(id, json!({ "tools": tools() })),
        "tools/call" => {
            let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            match call(app, name, args).await {
                Ok(v) => {
                    let text = if let Some(t) = v.as_str() {
                        t.to_string()
                    } else {
                        serde_json::to_string_pretty(&v).unwrap_or_default()
                    };
                    rpc_ok(
                        id,
                        json!({ "content": [{ "type": "text", "text": text }], "structuredContent": v, "isError": false }),
                    )
                }
                Err(e) => rpc_ok(
                    id,
                    json!({ "content": [{ "type": "text", "text": e }], "isError": true }),
                ),
            }
        }
        "resources/list" => rpc_ok(id, json!({ "resources": [] })),
        "prompts/list" => rpc_ok(id, json!({ "prompts": [] })),
        _ => rpc_error(id, -32601, format!("method not found: {}", method)),
    })
}

/// Corpo inteiro (mensagem única ou lote) → resposta pronta para o HTTP.
pub async fn handle_body(app: &AppHandle, body: &Value) -> Option<Value> {
    match body {
        Value::Array(items) => {
            let mut out = Vec::new();
            for m in items {
                if let Some(r) = handle(app, m).await {
                    out.push(r);
                }
            }
            if out.is_empty() {
                None
            } else {
                Some(Value::Array(out))
            }
        }
        m => handle(app, m).await,
    }
}

/// Trechos de configuração para os clientes, com a URL e o token deste app.
pub fn client_snippets(url: &str, token: &str) -> Vec<(String, String)> {
    let auth = format!("Authorization: Bearer {}", token);
    vec![
        ("Claude Code".into(), format!("claude mcp add --transport http omniget {} --header \"{}\"", url, auth)),
        (
            "Cursor".into(),
            serde_json::to_string_pretty(&json!({ "mcpServers": { "omniget": { "url": url, "headers": { "Authorization": format!("Bearer {}", token) } } } })).unwrap_or_default(),
        ),
        (
            "VS Code".into(),
            serde_json::to_string_pretty(&json!({ "servers": { "omniget": { "type": "http", "url": url, "headers": { "Authorization": format!("Bearer {}", token) } } } })).unwrap_or_default(),
        ),
        (
            "Goose".into(),
            format!("extensions:\n  omniget:\n    enabled: true\n    type: streamable_http\n    name: omniget\n    uri: {}\n    headers:\n      Authorization: Bearer {}\n    timeout: 300", url, token),
        ),
        (
            "Claude Desktop".into(),
            serde_json::to_string_pretty(&json!({ "mcpServers": { "omniget": { "command": "npx", "args": ["-y", "mcp-remote", url, "--header", auth] } } })).unwrap_or_default(),
        ),
        (
            "Codex".into(),
            format!("[mcp_servers.omniget]\nurl = \"{}\"\nhttp_headers = {{ Authorization = \"Bearer {}\" }}", url, token),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!(
            "omniget-mcp-{}-{}-{}",
            name,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::canonicalize(d).unwrap()
    }

    #[test]
    fn tool_writes_stay_inside_the_download_folder() {
        let base = scratch("confine");
        let root = base.join("Downloads");
        let outside = base.join("Startup");
        std::fs::create_dir_all(&outside).unwrap();

        assert_eq!(confine_write_path("", &root, None).unwrap(), root);
        assert_eq!(
            confine_write_path("", &root, Some("merged.pdf")).unwrap(),
            root.join("merged.pdf")
        );
        assert_eq!(
            confine_write_path("pdf/out", &root, None).unwrap(),
            root.join("pdf").join("out")
        );
        let inside = root.join("a").join("b.mp3");
        assert_eq!(
            confine_write_path(inside.to_str().unwrap(), &root, None).unwrap(),
            inside
        );

        for bad in [
            outside.join("x.bat").to_string_lossy().to_string(),
            outside.to_string_lossy().to_string(),
            "../Startup".to_string(),
            root.join("..")
                .join("Startup")
                .to_string_lossy()
                .to_string(),
        ] {
            assert!(confine_write_path(&bad, &root, None).is_err(), "{bad}");
        }
        assert!(confine_write_path("x", std::path::Path::new(""), None).is_err());

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&outside, root.join("link")).unwrap();
            assert!(confine_write_path("link/x.bat", &root, None).is_err());
        }
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn transcribe_writes_into_the_download_folder() {
        let base = scratch("transcribe");
        let root = base.join("Downloads");
        let docs = base.join("Documents");
        std::fs::create_dir_all(&docs).unwrap();
        let input = docs.join("notes.mp3").to_string_lossy().to_string();
        let opts = transcribe_options(
            &json!({ "input": input }),
            "base".into(),
            "auto".into(),
            &root,
        )
        .unwrap();
        assert_eq!(std::path::PathBuf::from(&opts.output_dir), root);
        let outside = docs.to_string_lossy().to_string();
        assert!(transcribe_options(
            &json!({ "input": input, "output_dir": outside }),
            "base".into(),
            "auto".into(),
            &root,
        )
        .is_err());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn schemas_are_objects() {
        let list = tools();
        assert!(list.len() > 20);
        for t in &list {
            assert_eq!(t.input_schema["type"], "object", "{}", t.name);
            assert!(!t.description.is_empty());
            // Todo campo obrigatorio precisa existir em `properties`.
            let props = t.input_schema["properties"]
                .as_object()
                .unwrap_or_else(|| panic!("{} sem properties", t.name));
            let required = t.input_schema["required"]
                .as_array()
                .unwrap_or_else(|| panic!("{} sem required", t.name));
            for r in required {
                let key = r.as_str().unwrap_or_default();
                assert!(props.contains_key(key), "{}: required {:?}", t.name, key);
            }
        }
        let names: std::collections::HashSet<_> = list.iter().map(|t| t.name).collect();
        assert_eq!(names.len(), list.len(), "nomes repetidos");

        // Controle da fila de downloads (mcp-downloads-control).
        for name in [
            "download_enqueue",
            "downloads_queue",
            "download_status",
            "download_cancel",
            "download_pause",
            "download_resume",
        ] {
            assert!(names.contains(name), "faltou a tool {}", name);
        }
        let by_name = |n: &str| {
            list.iter()
                .find(|t| t.name == n)
                .unwrap_or_else(|| panic!("sem {}", n))
        };
        assert_eq!(
            by_name("download_enqueue").input_schema["required"],
            json!(["url"])
        );
        assert_eq!(
            by_name("download_enqueue").input_schema["properties"]["mode"]["enum"],
            json!(["video", "audio"])
        );
        // Listar a fila nao exige argumento nenhum.
        assert_eq!(
            by_name("downloads_queue").input_schema["required"],
            json!([])
        );
        for n in [
            "download_status",
            "download_cancel",
            "download_pause",
            "download_resume",
        ] {
            assert_eq!(
                by_name(n).input_schema["required"],
                json!(["download_id"]),
                "{}",
                n
            );
            assert_eq!(
                by_name(n).input_schema["properties"]["download_id"]["type"],
                "integer",
                "{}",
                n
            );
        }
        // O filtro de status da lista tem que casar com o que `status_key` devolve.
        let allowed = by_name("downloads_queue").input_schema["properties"]["status"]["enum"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let allowed: Vec<String> = allowed
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
        assert_eq!(allowed, QUEUE_STATUS_KEYS.to_vec());
    }

    #[test]
    fn argumento_errado_da_erro_legivel() {
        let a = json!({ "download_id": "42", "url": "  " });
        assert_eq!(need_id(&a, "download_id"), Ok(42));
        let Err(e) = need_id(&json!({}), "download_id") else {
            panic!("id faltando deveria falhar");
        };
        assert!(e.contains("download_id") && e.contains("integer"), "{}", e);
        let Err(e) = need_str(&a, "url") else {
            panic!("url em branco deveria falhar");
        };
        assert!(e.contains("url") && e.contains("required"), "{}", e);
        let Err(e) = need_str(&json!({}), "query") else {
            panic!("query faltando deveria falhar");
        };
        assert!(e.contains("query"), "{}", e);
    }

    #[test]
    fn status_da_fila_vira_chave_estavel() {
        use crate::core::queue::QueueStatus as S;
        assert_eq!(status_key(&S::Queued), "queued");
        assert_eq!(status_key(&S::Active), "active");
        assert_eq!(status_key(&S::Paused), "paused");
        assert_eq!(status_key(&S::Seeding), "seeding");
        assert_eq!(status_key(&S::Complete { success: true }), "complete");
        assert_eq!(status_key(&S::Complete { success: false }), "complete");
        assert_eq!(
            status_key(&S::Error {
                message: "Cancelled".into(),
                retryable: false
            }),
            "error"
        );
    }

    #[test]
    fn snippets_carry_token() {
        for (_, s) in client_snippets("http://127.0.0.1:47720/mcp", "tok123") {
            assert!(s.contains("tok123"));
            assert!(s.contains("47720"));
        }
    }
}
