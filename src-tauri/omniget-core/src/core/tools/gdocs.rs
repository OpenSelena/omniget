//! Google Docs/Slides/Sheets públicos (estudo 51): o export oficial
//! `…/d/{id}/export?format=` dispensa API e OAuth para links compartilhados.

use std::path::PathBuf;

use anyhow::anyhow;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct GdocInfo {
    pub kind: String,
    pub id: String,
    pub formats: Vec<String>,
    #[serde(default)]
    pub is_published: bool,
}

pub fn parse(url: &str) -> Option<GdocInfo> {
    let pub_re = regex::Regex::new(
        r"docs\.google\.com/(document|presentation|spreadsheets)/d/e/([A-Za-z0-9_-]+)",
    )
    .ok()?;
    if let Some(c) = pub_re.captures(url) {
        let kind = c[1].to_string();
        let formats = match kind.as_str() {
            "presentation" => vec!["pdf", "svg"],
            "document" => vec!["pdf", "html"],
            _ => vec!["pdf", "csv"],
        };
        return Some(GdocInfo {
            kind,
            id: c[2].to_string(),
            formats: formats.into_iter().map(String::from).collect(),
            is_published: true,
        });
    }

    let re = regex::Regex::new(
        r"docs\.google\.com/(document|presentation|spreadsheets)/d/([A-Za-z0-9_-]+)",
    )
    .ok()?;
    let c = re.captures(url)?;
    if &c[2] == "e" {
        return None;
    }
    let kind = c[1].to_string();
    let formats = match kind.as_str() {
        "document" => vec!["pdf", "docx", "odt", "txt", "epub", "html"],
        "presentation" => vec!["pdf", "pptx", "odp", "txt", "svg"],
        _ => vec!["pdf", "xlsx", "ods", "csv"],
    };
    Some(GdocInfo {
        kind,
        id: c[2].to_string(),
        formats: formats.into_iter().map(String::from).collect(),
        is_published: false,
    })
}

pub fn export_url(info: &GdocInfo, format: &str) -> String {
    format!(
        "https://docs.google.com/{}/d/{}/export?format={}",
        info.kind, info.id, format
    )
}

fn filename_from_disposition(v: &str) -> Option<String> {
    // filename*=UTF-8''nome.pdf  ou  filename="nome.pdf"
    if let Some(i) = v.find("filename*=UTF-8''") {
        let s = &v[i + 17..];
        let end = s.find(';').unwrap_or(s.len());
        return urlencoding::decode(&s[..end]).ok().map(|c| c.to_string());
    }
    let i = v.find("filename=")?;
    let s = v[i + 9..].trim().trim_matches('"');
    let end = s.find(';').unwrap_or(s.len());
    Some(s[..end].trim_matches('"').to_string())
}

pub async fn download(
    url: &str,
    format: &str,
    dest_dir: &str,
    progress: super::ProgressFn,
) -> anyhow::Result<String> {
    let info = parse(url).ok_or_else(|| {
        anyhow!("cole um link de docs.google.com (Documentos, Apresentações ou Planilhas)")
    })?;
    if !info.formats.iter().any(|f| f == format) {
        return Err(anyhow!(
            "formato {} nao disponivel para {}",
            format,
            info.kind
        ));
    }

    if info.kind == "presentation" && (info.is_published || format == "svg") {
        let res = super::gslides::download_presentation(url, format, dest_dir, progress).await?;
        return Ok(res.output_path);
    }

    let client = super::gslides::build_google_client(Some(url)).or_else(|_| super::client())?;
    let export = export_url(&info, format);
    let resp = client.get(&export).send().await?;
    let status = resp.status();
    let is_html_redirect = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|ct| ct.to_ascii_lowercase().starts_with("text/html") && format != "html")
        .unwrap_or(false);

    if (!status.is_success() || is_html_redirect) && info.kind == "presentation" && format == "pdf" {
        if let Ok(res) =
            super::gslides::download_presentation(url, "pdf", dest_dir, progress.clone()).await
        {
            return Ok(res.output_path);
        }
    }

    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Err(anyhow!(
            "o arquivo nao e publico; abra no navegador e use Arquivo > Fazer download"
        ));
    }
    if !status.is_success() {
        return Err(anyhow!("Google Docs: HTTP {}", status));
    }
    let name = resp
        .headers()
        .get(reqwest::header::CONTENT_DISPOSITION)
        .and_then(|v| v.to_str().ok())
        .and_then(filename_from_disposition)
        .unwrap_or_else(|| format!("{}.{}", info.id, format));
    let dir = PathBuf::from(dest_dir);
    std::fs::create_dir_all(&dir)?;
    let dest = dir.join(super::sanitize_name(&name));
    let bytes = resp.bytes().await?;
    tokio::fs::write(&dest, &bytes).await?;
    super::report(
        &progress,
        "gdocs",
        "done",
        bytes.len() as u64,
        Some(bytes.len() as u64),
        None,
    );
    Ok(dest.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_kinds() {
        let i = parse("https://docs.google.com/presentation/d/1AbC_d-e/edit#slide=1").unwrap();
        assert_eq!(i.kind, "presentation");
        assert_eq!(i.id, "1AbC_d-e");
        assert!(!i.is_published);
        assert!(parse("https://drive.google.com/x").is_none());
        assert_eq!(
            export_url(&i, "pdf"),
            "https://docs.google.com/presentation/d/1AbC_d-e/export?format=pdf"
        );

        let pub_i = parse("https://docs.google.com/presentation/d/e/2PACX-1vMockDummyId1234567890abcdefghijklmnopqrstuvwxyz/pubembed?start=false&loop=false").unwrap();
        assert_eq!(pub_i.kind, "presentation");
        assert!(pub_i.is_published);
        assert_eq!(
            pub_i.id,
            "2PACX-1vMockDummyId1234567890abcdefghijklmnopqrstuvwxyz"
        );
        assert_eq!(pub_i.formats, vec!["pdf".to_string(), "svg".to_string()]);
    }

    #[test]
    fn disposition() {
        assert_eq!(
            filename_from_disposition("attachment; filename=\"a b.pdf\"").as_deref(),
            Some("a b.pdf")
        );
        assert_eq!(
            filename_from_disposition("attachment; filename*=UTF-8''caf%C3%A9.pdf").as_deref(),
            Some("café.pdf")
        );
    }
}
