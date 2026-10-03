use omniget_core::core::ai::{self, AiConfigView, AiHistoryEntry, AiProvider, KeyAction};
use serde::Serialize;

const MAX_TRANSCRIPT_CHARS: usize = 12000;

#[tauri::command]
pub fn ai_get_config() -> AiConfigView {
    ai::get().view()
}

// A tela manda o provedor da tabela (`kind`) junto do `provider` genérico, e a
// intenção sobre a chave em `key_action`: "keep" mantém a guardada, "clear"
// apaga (troca de provedor com o campo vazio) e "set" grava a digitada. O
// segredo nunca volta para a UI, então a intenção precisa ser explícita.
#[tauri::command]
pub fn ai_set_config(
    provider: AiProvider,
    kind: Option<String>,
    model: String,
    local_base_url: String,
    key_action: Option<String>,
    key: Option<String>,
) -> Result<AiConfigView, String> {
    let action = match key_action.as_deref() {
        None | Some("keep") => KeyAction::Keep,
        Some("clear") => KeyAction::Clear,
        Some("set") => match key.as_deref().map(str::trim) {
            Some(k) if !k.is_empty() => KeyAction::Set(k),
            _ => KeyAction::Clear,
        },
        Some(other) => return Err(format!("unknown key_action: {}", other)),
    };
    let kind = kind.unwrap_or_default();
    if provider == AiProvider::None || kind.trim() == "none" {
        return Ok(ai::clear().view());
    }
    // Cliente antigo (sem `kind`): mantém o caminho de antes.
    if kind.trim().is_empty() {
        return Ok(ai::set(provider, model, local_base_url, None, None).view());
    }
    Ok(ai::set_with_kind(&kind, model, local_base_url, action).view())
}

#[tauri::command]
pub async fn ai_test() -> Result<String, String> {
    ai::chat(
        "You are a connectivity test. Reply with the single word: ok.",
        "ping",
    )
    .await
}

#[derive(Clone, Serialize)]
pub struct AiSummaryResult {
    pub title: String,
    pub summary: String,
}

#[cfg(not(target_os = "android"))]
async fn transcribe_via_audio(
    ytdlp: &std::path::Path,
    url: &str,
    tmp: &std::path::Path,
) -> Result<String, String> {
    let audio_tmpl = tmp.join("audio.%(ext)s");
    let _ = omniget_core::core::ytdlp::ytdlp_command(ytdlp)
        .arg("-f")
        .arg("bestaudio/best")
        .arg("--no-warnings")
        .arg("-o")
        .arg(audio_tmpl.to_string_lossy().to_string())
        .arg(url)
        .output()
        .await
        .map_err(|e| format!("yt-dlp audio failed: {}", e))?;

    let mut src: Option<(u64, std::path::PathBuf)> = None;
    let mut rd = tokio::fs::read_dir(tmp).await.map_err(|e| e.to_string())?;
    while let Ok(Some(entry)) = rd.next_entry().await {
        let p = entry.path();
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if name.starts_with("audio.") {
            let len = entry.metadata().await.map(|m| m.len()).unwrap_or(0);
            if src.as_ref().map(|(l, _)| len > *l).unwrap_or(true) {
                src = Some((len, p));
            }
        }
    }
    let (_, src_path) = src.ok_or_else(|| "no audio".to_string())?;

    let ffmpeg = omniget_core::core::dependencies::find_tool("ffmpeg")
        .await
        .ok_or_else(|| "ffmpeg unavailable".to_string())?;
    let small = tmp.join("a16.mp3");
    let status = omniget_core::core::process::command(&ffmpeg)
        .arg("-y")
        .arg("-i")
        .arg(src_path.to_string_lossy().to_string())
        .arg("-ac")
        .arg("1")
        .arg("-ar")
        .arg("16000")
        .arg("-b:a")
        .arg("32k")
        .arg(small.to_string_lossy().to_string())
        .status()
        .await
        .map_err(|e| format!("ffmpeg failed: {}", e))?;
    if !status.success() {
        return Err("ffmpeg transcode failed".to_string());
    }

    omniget_core::core::ai::transcribe(&small).await
}

#[cfg(not(target_os = "android"))]
async fn fetch_transcript(url: &str) -> Result<(String, String), String> {
    let ytdlp = crate::core::ytdlp::find_ytdlp_cached()
        .await
        .ok_or_else(|| "yt-dlp unavailable".to_string())?;

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let tmp = std::env::temp_dir().join(format!("omniget-ai-{stamp}"));
    tokio::fs::create_dir_all(&tmp)
        .await
        .map_err(|e| e.to_string())?;

    let out_template = tmp.join("t.%(ext)s");
    let result: Result<(String, String), String> = (|| async {
        let output = omniget_core::core::ytdlp::ytdlp_command(&ytdlp)
            .arg("--skip-download")
            .arg("--write-subs")
            .arg("--write-auto-subs")
            .arg("--sub-langs")
            .arg("en.*,en,.*-orig,.*")
            .arg("--convert-subs")
            .arg("srt")
            .arg("--no-warnings")
            .arg("--print")
            .arg("title")
            .arg("-o")
            .arg(out_template.to_string_lossy().to_string())
            .arg(url)
            .output()
            .await
            .map_err(|e| format!("yt-dlp failed: {}", e))?;

        let title = String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string();

        let mut best: Option<(u64, std::path::PathBuf)> = None;
        let mut rd = tokio::fs::read_dir(&tmp).await.map_err(|e| e.to_string())?;
        while let Ok(Some(entry)) = rd.next_entry().await {
            let p = entry.path();
            let ext = p
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if ext == "srt" || ext == "vtt" {
                let len = entry.metadata().await.map(|m| m.len()).unwrap_or(0);
                if best.as_ref().map(|(l, _)| len > *l).unwrap_or(true) {
                    best = Some((len, p));
                }
            }
        }

        if let Some((_, sub_path)) = best {
            let content = tokio::fs::read_to_string(&sub_path)
                .await
                .map_err(|e| e.to_string())?;
            let transcript = omniget_core::core::subtitle_merge::extract_transcript(&content);
            if !transcript.trim().is_empty() {
                return Ok((title, transcript));
            }
        }

        // No usable subtitles — fall back to Whisper-style audio transcription
        // (only works on OpenAI-compatible providers; surfaces as no_transcript
        // otherwise so the UI hint stays accurate).
        match transcribe_via_audio(&ytdlp, url, &tmp).await {
            Ok(t) if !t.trim().is_empty() => Ok((title, t)),
            _ => Err("no_transcript".to_string()),
        }
    })()
    .await;

    let _ = tokio::fs::remove_dir_all(&tmp).await;
    result
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn ai_summarize_url(
    url: String,
    style: Option<String>,
    lang: Option<String>,
) -> Result<AiSummaryResult, String> {
    if !ai::get().is_configured() {
        return Err("ai_not_configured".to_string());
    }
    let (title, mut transcript) = fetch_transcript(&url).await?;
    if transcript.chars().count() > MAX_TRANSCRIPT_CHARS {
        transcript = transcript.chars().take(MAX_TRANSCRIPT_CHARS).collect();
    }
    let shape = match style.as_deref() {
        Some("short") => "in one short paragraph (no bullet points)",
        Some("detailed") => "as a thorough overview with sections and 8-12 detailed bullet points",
        _ => "as a short paragraph followed by 3-6 key bullet points",
    };
    let language = match lang.as_deref().map(str::trim) {
        Some(l) if !l.is_empty() => format!("Write the summary in {}.", l),
        _ => "Write the summary in the transcript's own language.".to_string(),
    };
    let system = format!(
        "You are a concise assistant. Summarize the following video transcript {}. {}",
        shape, language
    );
    let summary = ai::chat(&system, &transcript).await?;
    ai::history_add("summary", &url, &title, &summary);
    Ok(AiSummaryResult { title, summary })
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn whisper_generate(url: String) -> Result<String, String> {
    let cfg = ai::get();
    if !matches!(cfg.provider, AiProvider::Openai | AiProvider::Local) {
        return Err("ai_not_configured".to_string());
    }
    let ytdlp = crate::core::ytdlp::find_ytdlp_cached()
        .await
        .ok_or_else(|| "yt-dlp unavailable".to_string())?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let tmp = std::env::temp_dir().join(format!("omniget-whisper-{stamp}"));
    tokio::fs::create_dir_all(&tmp)
        .await
        .map_err(|e| e.to_string())?;
    let result = transcribe_via_audio(&ytdlp, &url, &tmp).await;
    let _ = tokio::fs::remove_dir_all(&tmp).await;
    let transcript = result?;
    ai::history_add("transcript", &url, "", &transcript);
    Ok(transcript)
}

#[tauri::command]
pub fn ai_history_list() -> Vec<AiHistoryEntry> {
    ai::history_list()
}

#[tauri::command]
pub fn ai_history_clear() {
    ai::history_clear();
}
