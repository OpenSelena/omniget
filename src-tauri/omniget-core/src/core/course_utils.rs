use std::path::Path;

use anyhow::anyhow;
use tokio_util::sync::CancellationToken;

pub async fn save_description(dir: &str, content: &str, format: &str) -> anyhow::Result<()> {
    if content.trim().is_empty() {
        return Ok(());
    }

    let ext = match format {
        "markdown" | "md" => "md",
        "text" | "txt" => "txt",
        _ => "html",
    };

    let path = format!("{}/description.{}", dir, ext);

    if Path::new(&path).exists() {
        return Ok(());
    }

    let wrapped = if ext == "html" && !content.trim_start().starts_with("<!") && !content.trim_start().starts_with("<html") {
        format!(
            "<!DOCTYPE html>\n<html>\n<head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><style>body{{max-width:800px;margin:40px auto;padding:0 20px;font-family:system-ui,sans-serif;line-height:1.6;color:#333}}img{{max-width:100%;height:auto}}a{{color:#0066cc}}</style></head>\n<body>\n{}\n</body>\n</html>",
            content
        )
    } else {
        content.to_string()
    };

    std::fs::write(&path, wrapped.as_bytes())?;
    tracing::debug!("[course] saved description: {}", path);
    Ok(())
}

pub async fn download_attachment(
    client: &reqwest::Client,
    url: &str,
    dir: &str,
    name: &str,
    cancel_token: &CancellationToken,
) -> anyhow::Result<u64> {
    if url.is_empty() || name.is_empty() {
        return Ok(0);
    }

    let sanitized = sanitize_filename::sanitize(name);
    let filename = if sanitized.is_empty() {
        let ext = url
            .rsplit('.')
            .next()
            .and_then(|e| e.split('?').next())
            .filter(|e| e.len() <= 5)
            .unwrap_or("bin");
        format!("attachment.{}", ext)
    } else {
        sanitized
    };

    let path = format!("{}/{}", dir, filename);

    if Path::new(&path).exists() {
        let meta = std::fs::metadata(&path);
        if meta.map(|m| m.len() > 0).unwrap_or(false) {
            return Ok(0);
        }
    }

    if cancel_token.is_cancelled() {
        return Err(anyhow!("Download cancelled"));
    }

    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| anyhow!("Failed to download attachment: {}", e))?;

    if !resp.status().is_success() {
        return Err(anyhow!(
            "Attachment download failed: HTTP {}",
            resp.status()
        ));
    }

    let bytes = resp.bytes().await?;
    let size = bytes.len() as u64;

    if size == 0 {
        return Ok(0);
    }

    let part_path = format!("{}.part", path);
    std::fs::write(&part_path, &bytes)?;
    std::fs::rename(&part_path, &path)?;

    tracing::debug!("[course] attachment saved: {} ({} bytes)", path, size);
    Ok(size)
}

pub async fn mark_course_complete(course_dir: &str) -> anyhow::Result<()> {
    let marker = format!("{}/.complete", course_dir);
    std::fs::write(&marker, "done")?;
    tracing::info!("[course] marked complete: {}", course_dir);
    Ok(())
}

pub fn is_course_complete(course_dir: &str) -> bool {
    Path::new(&format!("{}/.complete", course_dir)).exists()
}

pub async fn ensure_dir(path: &str) -> anyhow::Result<()> {
    std::fs::create_dir_all(path)?;
    Ok(())
}
