//! Extractor y compilador vectorial de Google Slides a PDF panorámico (16:9).
//!
//! Soporta:
//! 1. Enlaces publicados en la web (`/presentation/d/e/2PACX-.../pubembed` o `/pub`),
//!    los cuales no disponen de `/export?format=pdf` (devuelven HTTP 404).
//! 2. Presentaciones restringidas donde `/export?format=pdf` devuelve 401/403 pero el
//!    visor (`/embed` o `/htmlpresent`) entrega las diapositivas en bloques `SK_svgData`.
//! 3. Compilación local de carpetas de archivos `.svg` ordenados numéricamente (`resvg` +
//!    `tiny-skia` -> PDF 16:9 de `1280 x 720 pt`) y recepción desde el script JS del
//!    navegador a través del `local_bridge`.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use anyhow::{anyhow, Context};
use base64::Engine;
use futures::stream::{self, StreamExt};
use image::codecs::jpeg::JpegEncoder;
use image::ColorType;
use regex::Regex;
use resvg::{tiny_skia, usvg};
use serde::{Deserialize, Serialize};

pub const SLIDE_WIDTH_PT: f64 = 1280.0;
pub const SLIDE_HEIGHT_PT: f64 = 720.0;

/// Resolución de rasterizado en píxeles (Full HD 16:9) sobre página física de 1280x720 pt.
const RENDER_WIDTH_PX: u32 = 1920;
const RENDER_HEIGHT_PX: u32 = 1080;
const JPEG_QUALITY: u8 = 92;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GslidesTarget {
    pub id: String,
    pub is_published: bool,
}

impl GslidesTarget {
    pub fn viewer_url(&self) -> String {
        if self.is_published {
            format!(
                "https://docs.google.com/presentation/d/e/{}/pubembed?start=false&loop=false",
                self.id
            )
        } else {
            format!(
                "https://docs.google.com/presentation/d/{}/embed?start=false&loop=false",
                self.id
            )
        }
    }

    pub fn htmlpresent_url(&self) -> String {
        if self.is_published {
            format!("https://docs.google.com/presentation/d/e/{}/pub", self.id)
        } else {
            format!(
                "https://docs.google.com/presentation/d/{}/htmlpresent",
                self.id
            )
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GslidesDownloadResult {
    pub title: String,
    pub pages: usize,
    pub output_path: String,
    pub format: String,
}

#[derive(Debug, Clone)]
pub struct ExtractedPresentation {
    pub title: String,
    pub svgs: Vec<String>,
}

/// Reconoce tanto `/presentation/d/e/2PACX-...` como `/presentation/d/{id}`.
pub fn parse_gslides_url(url: &str) -> Option<GslidesTarget> {
    static PUB_RE: OnceLock<Regex> = OnceLock::new();
    static DOC_RE: OnceLock<Regex> = OnceLock::new();

    let pub_re = PUB_RE.get_or_init(|| {
        Regex::new(r"docs\.google\.com/presentation/d/e/([A-Za-z0-9_-]+)").unwrap()
    });
    if let Some(c) = pub_re.captures(url) {
        return Some(GslidesTarget {
            id: c[1].to_string(),
            is_published: true,
        });
    }

    let doc_re = DOC_RE
        .get_or_init(|| Regex::new(r"docs\.google\.com/presentation/d/([A-Za-z0-9_-]+)").unwrap());
    let c = doc_re.captures(url)?;
    let id = &c[1];
    if id == "e" {
        return None;
    }
    Some(GslidesTarget {
        id: id.to_string(),
        is_published: false,
    })
}

/// Decodifica cadenas literales de JavaScript como las que emite Google Slides
/// en `SK_svgData = '\x3csvg version\x3d\x221.1\x22 ...'`.
pub fn unescape_js_string(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let bytes = raw.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            match bytes[i + 1] {
                b'x' if i + 3 < bytes.len() => {
                    if let Ok(hex_str) = std::str::from_utf8(&bytes[i + 2..i + 4]) {
                        if let Ok(val) = u8::from_str_radix(hex_str, 16) {
                            out.push(val as char);
                            i += 4;
                            continue;
                        }
                    }
                }
                b'u' if i + 5 < bytes.len() => {
                    if let Ok(hex_str) = std::str::from_utf8(&bytes[i + 2..i + 6]) {
                        if let Ok(val) = u32::from_str_radix(hex_str, 16) {
                            if let Some(ch) = char::from_u32(val) {
                                out.push(ch);
                                i += 6;
                                continue;
                            }
                        }
                    }
                }
                b'/' => {
                    out.push('/');
                    i += 2;
                    continue;
                }
                b'\\' => {
                    out.push('\\');
                    i += 2;
                    continue;
                }
                b'\'' => {
                    out.push('\'');
                    i += 2;
                    continue;
                }
                b'"' => {
                    out.push('"');
                    i += 2;
                    continue;
                }
                b'n' => {
                    out.push('\n');
                    i += 2;
                    continue;
                }
                b'r' => {
                    out.push('\r');
                    i += 2;
                    continue;
                }
                b't' => {
                    out.push('\t');
                    i += 2;
                    continue;
                }
                _ => {}
            }
        }
        // Avanza un carácter UTF-8 válido
        let rest = &raw[i..];
        if let Some(ch) = rest.chars().next() {
            out.push(ch);
            i += ch.len_utf8();
        } else {
            break;
        }
    }
    out
}

/// Fija explícitamente `width="1280px" height="720px" viewBox="0 0 1280 720"`
/// en el elemento `<svg>` raíz si aún no tiene `width`/`height` explícitos,
/// preservando el `viewBox` original si ya es proporcional o asignando `0 0 1280 720`.
pub fn normalize_slide_svg_geometry(svg: &str) -> String {
    let trimmed = svg.trim();
    let Some(svg_start) = trimmed.find("<svg") else {
        return trimmed.to_string();
    };
    let Some(tag_end_rel) = trimmed[svg_start..].find('>') else {
        return trimmed.to_string();
    };
    let tag_end = svg_start + tag_end_rel;
    let open_tag = &trimmed[svg_start..=tag_end];

    static WIDTH_RE: OnceLock<Regex> = OnceLock::new();
    static HEIGHT_RE: OnceLock<Regex> = OnceLock::new();
    static VIEWBOX_RE: OnceLock<Regex> = OnceLock::new();

    let width_re = WIDTH_RE.get_or_init(|| Regex::new(r#"\bwidth\s*=\s*"[^"]*""#).unwrap());
    let height_re = HEIGHT_RE.get_or_init(|| Regex::new(r#"\bheight\s*=\s*"[^"]*""#).unwrap());
    let viewbox_re = VIEWBOX_RE.get_or_init(|| Regex::new(r#"\bviewBox\s*=\s*"[^"]*""#).unwrap());

    let mut cleaned = width_re.replace_all(open_tag, "").into_owned();
    cleaned = height_re.replace_all(&cleaned, "").into_owned();
    if !viewbox_re.is_match(&cleaned) {
        cleaned = cleaned.replacen(
            "<svg",
            r#"<svg viewBox="0 0 1280 720""#,
            1,
        );
    }
    cleaned = cleaned.replacen(
        "<svg",
        r#"<svg width="1280px" height="720px""#,
        1,
    );

    let mut result = String::with_capacity(trimmed.len() + 48);
    result.push_str(&trimmed[..svg_start]);
    result.push_str(&cleaned);
    result.push_str(&trimmed[tag_end + 1..]);
    result
}

fn clean_presentation_title(raw: &str) -> String {
    let mut t = raw
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">");
    for suffix in [
        " - Presentaciones de Google",
        " - Google Slides",
        " - Apresentações Google",
        " - Google 簡報",
        " - Google 幻灯片",
        " - Google スライド",
    ] {
        if let Some(stripped) = t.strip_suffix(suffix) {
            t = stripped.to_string();
        }
    }
    for ext in [".pptx", ".ppt", ".odp", ".pdf"] {
        if t.to_lowercase().ends_with(ext) && t.len() > ext.len() {
            t.truncate(t.len() - ext.len());
        }
    }
    let trimmed = t.trim();
    if trimmed.is_empty() {
        "presentacion_google_slides".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn extract_title_from_viewer_html(html: &str) -> String {
    static VIEWER_TITLE_RE: OnceLock<Regex> = OnceLock::new();
    static HTML_TITLE_RE: OnceLock<Regex> = OnceLock::new();

    let viewer_re = VIEWER_TITLE_RE
        .get_or_init(|| Regex::new(r"title:\s*'((?:\\.|[^'])*)'").unwrap());
    let head_slice = &html[..html.len().min(350_000)];
    if let Some(c) = viewer_re.captures(head_slice) {
        let decoded = unescape_js_string(&c[1]);
        if !decoded.trim().is_empty() {
            return clean_presentation_title(&decoded);
        }
    }

    let html_re = HTML_TITLE_RE
        .get_or_init(|| Regex::new(r"(?is)<title[^>]*>(.*?)</title>").unwrap());
    if let Some(c) = html_re.captures(head_slice) {
        let raw = c[1].trim();
        if !raw.is_empty() {
            return clean_presentation_title(raw);
        }
    }

    "presentacion_google_slides".to_string()
}

/// Extrae todos los bloques `SK_svgData` en el orden exacto de las diapositivas.
pub fn extract_viewer_slides(html: &str) -> ExtractedPresentation {
    static SVG_DATA_RE: OnceLock<Regex> = OnceLock::new();
    let re = SVG_DATA_RE
        .get_or_init(|| Regex::new(r"SK_svgData\s*=\s*'((?:\\.|[^'])*)'").unwrap());

    let title = extract_title_from_viewer_html(html);
    let mut svgs = Vec::new();
    for cap in re.captures_iter(html) {
        let decoded = unescape_js_string(&cap[1]);
        if decoded.contains("<svg") {
            svgs.push(normalize_slide_svg_geometry(&decoded));
        }
    }

    ExtractedPresentation { title, svgs }
}

/// Recolecta todas las URLs remotas `http(s)://` dentro de atributos `xlink:href` o `href`
/// en elementos `<image>`.
pub fn collect_remote_image_urls(svgs: &[String]) -> Vec<String> {
    static IMG_HREF_RE: OnceLock<Regex> = OnceLock::new();
    let re = IMG_HREF_RE.get_or_init(|| {
        Regex::new(r#"<image\b[^>]*?\b(?:xlink:)?href\s*=\s*"(https?://[^"]+)""#).unwrap()
    });

    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for svg in svgs {
        for cap in re.captures_iter(svg) {
            let url = cap[1].replace("&amp;", "&");
            if seen.insert(url.clone()) {
                out.push(url);
            }
        }
    }
    out
}

fn guess_image_mime(bytes: &[u8], content_type: Option<&str>) -> &'static str {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        return "image/png";
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return "image/jpeg";
    }
    if bytes.starts_with(b"GIF8") {
        return "image/gif";
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return "image/webp";
    }
    if let Some(ct) = content_type {
        let lower = ct.to_ascii_lowercase();
        if lower.contains("jpeg") || lower.contains("jpg") {
            return "image/jpeg";
        }
        if lower.contains("webp") {
            return "image/webp";
        }
        if lower.contains("gif") {
            return "image/gif";
        }
        if lower.contains("svg") {
            return "image/svg+xml";
        }
    }
    "image/png"
}

/// Descarga en paralelo las imágenes remotas referenciadas por los SVGs y las
/// incrusta como Data URLs (`data:<mime>;base64,...`) exactamente igual que
/// `svgImageToBase64` en el navegador, pero con deduplicación y concurrencia.
pub async fn embed_remote_images_base64(
    client: &reqwest::Client,
    svgs: &mut [String],
    progress: &super::ProgressFn,
    progress_id: &str,
) {
    let urls = collect_remote_image_urls(svgs);
    if urls.is_empty() {
        return;
    }

    let total_imgs = urls.len() as u64;
    super::report(
        progress,
        progress_id,
        "images",
        0,
        Some(total_imgs),
        Some(format!("Descargando {} recursos de imagen...", total_imgs)),
    );

    let fetched: Vec<(String, String)> = stream::iter(urls.into_iter().enumerate())
        .map(|(idx, url)| {
            let client = client.clone();
            let progress = progress.clone();
            let progress_id = progress_id.to_string();
            async move {
                let res = match client.get(&url).send().await {
                    Ok(r) if r.status().is_success() => {
                        let ct = r
                            .headers()
                            .get(reqwest::header::CONTENT_TYPE)
                            .and_then(|v| v.to_str().ok())
                            .map(|s| s.to_string());
                        match r.bytes().await {
                            Ok(bytes) if !bytes.is_empty() => {
                                let mime = guess_image_mime(&bytes, ct.as_deref());
                                let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                                Some((url, format!("data:{};base64,{}", mime, b64)))
                            }
                            _ => None,
                        }
                    }
                    _ => None,
                };
                if (idx + 1) % 5 == 0 || (idx + 1) as u64 == total_imgs {
                    super::report(
                        &progress,
                        &progress_id,
                        "images",
                        (idx + 1) as u64,
                        Some(total_imgs),
                        None,
                    );
                }
                res
            }
        })
        .buffer_unordered(12)
        .filter_map(|x| async move { x })
        .collect()
        .await;

    if fetched.is_empty() {
        return;
    }

    let map: HashMap<String, String> = fetched.into_iter().collect();

    for svg in svgs.iter_mut() {
        for (url, data_uri) in &map {
            if svg.contains(url) {
                *svg = svg.replace(url, data_uri);
            }
            let escaped = url.replace('&', "&amp;");
            if escaped != *url && svg.contains(&escaped) {
                *svg = svg.replace(&escaped, data_uri);
            }
        }
    }
}

fn shared_fontdb() -> Arc<usvg::fontdb::Database> {
    static FONTDB: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    FONTDB
        .get_or_init(|| {
            let mut db = usvg::fontdb::Database::new();
            db.load_system_fonts();
            Arc::new(db)
        })
        .clone()
}

/// Renderiza un SVG a un buffer JPEG en memoria con proporción 16:9 usando `resvg` y `tiny-skia`.
pub fn render_svg_to_jpeg(svg_str: &str) -> anyhow::Result<Vec<u8>> {
    let normalized = normalize_slide_svg_geometry(svg_str);
    let opt = usvg::Options {
        fontdb: shared_fontdb(),
        ..Default::default()
    };

    let tree = usvg::Tree::from_data(normalized.as_bytes(), &opt)
        .context("Error parseando el árbol SVG con usvg")?;

    let tree_size = tree.size();
    let target_w = RENDER_WIDTH_PX;
    let target_h = RENDER_HEIGHT_PX;

    let mut pixmap = tiny_skia::Pixmap::new(target_w, target_h)
        .ok_or_else(|| anyhow!("No se pudo reservar memoria para el pixmap {}x{}", target_w, target_h))?;

    // Fondo blanco por defecto para evitar transparencias negras al pasar a JPEG
    pixmap.fill(tiny_skia::Color::WHITE);

    let sx = target_w as f32 / tree_size.width().max(1.0);
    let sy = target_h as f32 / tree_size.height().max(1.0);
    let transform = tiny_skia::Transform::from_scale(sx, sy);

    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Convertir RGBA premultiplicado (tiny-skia) a RGB de 24 bits para JPEG
    let rgba = pixmap.data();
    let mut rgb = Vec::with_capacity((target_w * target_h * 3) as usize);
    for px in rgba.chunks_exact(4) {
        let a = px[3] as u16;
        if a == 255 {
            rgb.push(px[0]);
            rgb.push(px[1]);
            rgb.push(px[2]);
        } else if a == 0 {
            rgb.push(255);
            rgb.push(255);
            rgb.push(255);
        } else {
            // Des-premultiplicar sobre fondo blanco
            let r = ((px[0] as u16 * 255) / a).min(255) as u8;
            let g = ((px[1] as u16 * 255) / a).min(255) as u8;
            let b = ((px[2] as u16 * 255) / a).min(255) as u8;
            rgb.push(r);
            rgb.push(g);
            rgb.push(b);
        }
    }

    let mut jpeg_buf = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut jpeg_buf, JPEG_QUALITY);
    encoder
        .encode(&rgb, target_w, target_h, ColorType::Rgb8.into())
        .context("Error codificando diapositiva a JPEG")?;

    Ok(jpeg_buf)
}

/// Compila una lista ordenada de SVGs en un documento PDF 16:9 (`1280 x 720 pt`).
pub fn compile_svgs_to_pdf(
    svgs: &[String],
    progress: &super::ProgressFn,
    progress_id: &str,
) -> anyhow::Result<Vec<u8>> {
    if svgs.is_empty() {
        return Err(anyhow!("No hay diapositivas SVG para compilar"));
    }

    let total = svgs.len() as u64;
    let mut jpegs = Vec::with_capacity(svgs.len());

    for (idx, svg_str) in svgs.iter().enumerate() {
        super::report(
            progress,
            progress_id,
            "compile",
            idx as u64,
            Some(total),
            Some(format!("Renderizando diapositiva {}/{}...", idx + 1, total)),
        );
        let jpeg = render_svg_to_jpeg(svg_str)
            .with_context(|| format!("Error procesando diapositiva {}", idx + 1))?;
        jpegs.push(jpeg);
    }

    let pdf = super::jpeg_pdf::build_pdf_with_page_size(&jpegs, SLIDE_WIDTH_PT, SLIDE_HEIGHT_PT)?;
    super::report(progress, progress_id, "done", total, Some(total), None);
    Ok(pdf)
}

/// Extrae el primer número de un nombre de archivo para ordenamiento natural
/// (`diapositiva_1.svg`, `diapositiva_2.svg`, ..., `diapositiva_10.svg`).
pub fn extraer_numero_slide(nombre: &str) -> usize {
    static NUM_RE: OnceLock<Regex> = OnceLock::new();
    let re = NUM_RE.get_or_init(|| Regex::new(r"(\d+)").unwrap());
    re.captures(nombre)
        .and_then(|cap| cap.get(1))
        .and_then(|m| m.as_str().parse::<usize>().ok())
        .unwrap_or(0)
}

/// Lista todos los archivos `.svg` de un directorio en orden numérico natural.
pub fn obtener_svgs_ordenados(directorio: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut entradas: Vec<PathBuf> = std::fs::read_dir(directorio)
        .with_context(|| format!("No se pudo leer el directorio {}", directorio.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .map(|ext| ext.eq_ignore_ascii_case("svg"))
                .unwrap_or(false)
        })
        .collect();

    entradas.sort_by(|a, b| {
        let na = a.file_name().unwrap_or_default().to_string_lossy();
        let nb = b.file_name().unwrap_or_default().to_string_lossy();
        extraer_numero_slide(&na)
            .cmp(&extraer_numero_slide(&nb))
            .then_with(|| na.cmp(&nb))
    });

    Ok(entradas)
}

/// Compila todos los archivos `.svg` de una carpeta local en un PDF 16:9 (`1280 x 720 pt`).
pub async fn compile_svg_dir_to_pdf(
    svg_dir: &str,
    output_pdf: Option<&str>,
    progress: super::ProgressFn,
) -> anyhow::Result<GslidesDownloadResult> {
    let dir_path = PathBuf::from(svg_dir);
    let paths = obtener_svgs_ordenados(&dir_path)?;
    if paths.is_empty() {
        return Err(anyhow!(
            "No se encontraron archivos .svg en {}",
            dir_path.display()
        ));
    }

    let mut svgs = Vec::with_capacity(paths.len());
    for p in &paths {
        let content = tokio::fs::read_to_string(p)
            .await
            .with_context(|| format!("Error leyendo {}", p.display()))?;
        svgs.push(content);
    }

    let progress_id = format!("gslides-dir:{}", svg_dir);
    if let Ok(client) = build_google_client(None) {
        embed_remote_images_base64(&client, &mut svgs, &progress, &progress_id).await;
    }

    let dir_title = dir_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "presentacion_compilada".to_string());

    let out_path = match output_pdf.map(str::trim).filter(|s| !s.is_empty()) {
        Some(custom) => {
            let p = PathBuf::from(custom);
            if p.is_dir() {
                p.join(format!("{}.pdf", super::sanitize_name(&dir_title)))
            } else if p.extension().map(|e| e.eq_ignore_ascii_case("pdf")).unwrap_or(false) {
                p
            } else {
                p.with_extension("pdf")
            }
        }
        None => dir_path.join(format!("{}.pdf", super::sanitize_name(&dir_title))),
    };

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let svgs_for_compile = svgs;
    let progress_clone = progress.clone();
    let pid = progress_id.clone();
    let pdf_bytes = tokio::task::spawn_blocking(move || {
        compile_svgs_to_pdf(&svgs_for_compile, &progress_clone, &pid)
    })
    .await??;

    tokio::fs::write(&out_path, &pdf_bytes).await?;

    Ok(GslidesDownloadResult {
        title: dir_title,
        pages: paths.len(),
        output_path: out_path.to_string_lossy().to_string(),
        format: "pdf".to_string(),
    })
}

pub fn build_google_client(url_hint: Option<&str>) -> anyhow::Result<reqwest::Client> {
    use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
        ),
    );
    let mut builder = crate::core::http_client::apply_global_proxy(reqwest::Client::builder())
        .default_headers(headers)
        .timeout(std::time::Duration::from_secs(600));

    let probe_url = url_hint.unwrap_or("https://docs.google.com/presentation/");
    if let Some(jar) = crate::core::cookie_parser::load_extension_cookies_for_url(probe_url) {
        builder = builder.cookie_provider(jar);
    }
    Ok(builder.build()?)
}

/// Descarga y extrae las diapositivas de una presentación de Google Slides desde su visor
/// (`pubembed` / `embed` / `htmlpresent`), incrusta sus imágenes en Base64 y genera el
/// PDF 16:9 (`format == "pdf"`) o una carpeta con los `.svg` autocontenidos (`format == "svg"`).
pub async fn download_presentation(
    url: &str,
    format: &str,
    dest_dir: &str,
    progress: super::ProgressFn,
) -> anyhow::Result<GslidesDownloadResult> {
    let target = parse_gslides_url(url).ok_or_else(|| {
        anyhow!("Enlace de Google Slides no válido: {}", url)
    })?;

    let client = build_google_client(Some(url))?;
    let progress_id = format!("gslides:{}", target.id);

    super::report(
        &progress,
        &progress_id,
        "fetch",
        0,
        None,
        Some("Obteniendo visor de Google Slides...".to_string()),
    );

    let mut html = String::new();
    for candidate in [target.viewer_url(), target.htmlpresent_url()] {
        if let Ok(resp) = client.get(&candidate).send().await {
            if resp.status().is_success() {
                if let Ok(text) = resp.text().await {
                    if text.contains("SK_svgData") {
                        html = text;
                        break;
                    }
                }
            }
        }
    }

    if html.is_empty() {
        return Err(anyhow!(
            "No se encontraron diapositivas vectoriales en la presentación (puede requerir inicio de sesión o usar el extractor en navegador)."
        ));
    }

    let mut extracted = extract_viewer_slides(&html);
    drop(html);

    if extracted.svgs.is_empty() {
        return Err(anyhow!(
            "No se encontraron bloques SK_svgData en la presentación."
        ));
    }

    let pages = extracted.svgs.len();
    embed_remote_images_base64(&client, &mut extracted.svgs, &progress, &progress_id).await;

    let safe_title = super::sanitize_name(&extracted.title);
    let base_dir = PathBuf::from(dest_dir);
    std::fs::create_dir_all(&base_dir)?;

    if format.eq_ignore_ascii_case("svg") {
        let folder = base_dir.join(&safe_title);
        std::fs::create_dir_all(&folder)?;
        for (i, svg) in extracted.svgs.iter().enumerate() {
            let file_path = folder.join(format!("diapositiva_{:03}.svg", i + 1));
            tokio::fs::write(&file_path, svg.as_bytes()).await?;
        }
        super::report(
            &progress,
            &progress_id,
            "done",
            pages as u64,
            Some(pages as u64),
            None,
        );
        return Ok(GslidesDownloadResult {
            title: extracted.title,
            pages,
            output_path: folder.to_string_lossy().to_string(),
            format: "svg".to_string(),
        });
    }

    let svgs = extracted.svgs;
    let progress_clone = progress.clone();
    let pid = progress_id.clone();
    let pdf_bytes = tokio::task::spawn_blocking(move || {
        compile_svgs_to_pdf(&svgs, &progress_clone, &pid)
    })
    .await??;

    let pdf_path = base_dir.join(format!("{}.pdf", safe_title));
    tokio::fs::write(&pdf_path, &pdf_bytes).await?;

    Ok(GslidesDownloadResult {
        title: extracted.title,
        pages,
        output_path: pdf_path.to_string_lossy().to_string(),
        format: "pdf".to_string(),
    })
}

/// Genera el script JavaScript / Bookmarklet adaptado para enviar diapositivas al
/// `local_bridge` de OmniGet en `http://127.0.0.1:<port>/api`.
pub fn browser_extractor_script(port: u16) -> String {
    let tpl = r##"javascript:(function(){
  const SERVER_URL = "http://127.0.0.1:__PORT__/api";
  let count = 1;
  let lastSvgSignature = "";
  let duplicateCount = 0;
  const deckTitle = (document.title || "presentacion_google_slides").replace(/\s*-\s*(Presentaciones de Google|Google Slides|Apresentações Google).*$/i, "").trim();

  fetch(`${SERVER_URL}/reset`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ title: deckTitle })
  });

  let btn = document.createElement("button");
  btn.textContent = "⏳ Enviando slides a OmniGet...";
  btn.style.cssText = "position:fixed;top:20px;right:20px;z-index:999999;padding:14px 22px;background:#4f46e5;color:white;font-weight:bold;border:none;border-radius:8px;box-shadow:0 8px 24px rgba(0,0,0,0.35);font-family:system-ui,sans-serif;";
  document.body.appendChild(btn);

  async function svgImageToBase64(svgElement) {
    let images = svgElement.querySelectorAll("image");
    for (let img of images) {
      let href = img.getAttribute("xlink:href") || img.getAttribute("href");
      if (href && href.startsWith("http")) {
        try {
          let res = await fetch(href);
          let blob = await res.blob();
          let base64 = await new Promise(resolve => {
            let reader = new FileReader();
            reader.onloadend = () => resolve(reader.result);
            reader.readAsDataURL(blob);
          });
          img.setAttribute("xlink:href", base64);
          if (img.hasAttribute("href")) img.setAttribute("href", base64);
        } catch (e) {
          console.warn("Fallo CORS en imagen:", href);
        }
      }
    }
  }

  function advance() {
    let nextBtn = document.querySelector('[aria-label="Siguiente"]') ||
                  document.querySelector('[aria-label="Next"]') ||
                  document.querySelector('[aria-label="Próximo"]') ||
                  document.querySelector('.punch-viewer-nav-next');
    if (nextBtn) {
      nextBtn.click();
    } else {
      let target = document.querySelector('.punch-viewer-content') || document.body;
      target.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', keyCode: 39, bubbles: true }));
    }
  }

  async function stepCapture() {
    let svgs = document.querySelectorAll("svg");
    let target = null;
    svgs.forEach(s => {
      let vb = s.getAttribute("viewBox");
      if (vb && vb.includes("1280") && vb.includes("720")) target = s;
    });
    if (!target && svgs.length > 0) target = svgs[0];

    if (target) {
      let sig = target.innerHTML;
      if (sig === lastSvgSignature && count > 1) {
        if (++duplicateCount >= 2) {
          btn.textContent = "⚙️ Compilando PDF 16:9 en OmniGet...";
          let r = await fetch(`${SERVER_URL}/compile`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ title: deckTitle })
          });
          let j = await r.json().catch(() => ({}));
          btn.textContent = j.file ? `✅ PDF guardado: ${j.file}` : "✅ ¡PDF generado en OmniGet!";
          btn.style.background = "#10b981";
          setTimeout(() => btn.remove(), 6000);
          return;
        }
      } else {
        duplicateCount = 0;
        lastSvgSignature = sig;
      }

      let clone = target.cloneNode(true);
      clone.setAttribute("width", "1280px");
      clone.setAttribute("height", "720px");
      clone.setAttribute("viewBox", "0 0 1280 720");

      await svgImageToBase64(clone);

      let xml = new XMLSerializer().serializeToString(clone);
      await fetch(`${SERVER_URL}/slide`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ index: count++, svg_content: xml, title: deckTitle })
      });

      btn.textContent = `🚀 Enviado slide ${count - 1} a OmniGet...`;
    }

    advance();
    setTimeout(stepCapture, 1800);
  }

  setTimeout(stepCapture, 1200);
})();"##;
    tpl.replace("__PORT__", &port.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_published_and_standard_urls() {
        let pub_url = "https://docs.google.com/presentation/d/e/2PACX-1vMockDummyId1234567890abcdefghijklmnopqrstuvwxyz/pubembed?start=false&loop=false&delayms=3000&slide=id.p1";
        let t1 = parse_gslides_url(pub_url).unwrap();
        assert!(t1.is_published);
        assert_eq!(
            t1.id,
            "2PACX-1vMockDummyId1234567890abcdefghijklmnopqrstuvwxyz"
        );

        let std_url = "https://docs.google.com/presentation/d/1AbC_d-e/edit#slide=id.p1";
        let t2 = parse_gslides_url(std_url).unwrap();
        assert!(!t2.is_published);
        assert_eq!(t2.id, "1AbC_d-e");
    }

    #[test]
    fn unescapes_sk_svg_data_and_extracts_slides() {
        let sample_html = r#"
        <script>var viewerData = {title: 'Sample Presentation Deck.pptx', docData: []};</script>
        <script>SK_svgData = '\x3csvg version\x3d\x221.1\x22 viewBox\x3d\x220.0 0.0 1280.0 720.0\x22 xmlns\x3d\x22http:\/\/www.w3.org\/2000\/svg\x22\x3e\x3cpath fill\x3d\x22#ff0000\x22 d\x3d\x22M0 0h1280v720H0z\x22\/\x3e\x3c\/svg\x3e';</script>
        <script>SK_svgData = '\x3csvg version\x3d\x221.1\x22 viewBox\x3d\x220.0 0.0 1280.0 720.0\x22 xmlns\x3d\x22http:\/\/www.w3.org\/2000\/svg\x22\x3e\x3cpath fill\x3d\x22#00ff00\x22 d\x3d\x22M0 0h1280v720H0z\x22\/\x3e\x3c\/svg\x3e';</script>
        "#;
        let extracted = extract_viewer_slides(sample_html);
        assert_eq!(extracted.title, "Sample Presentation Deck");
        assert_eq!(extracted.svgs.len(), 2);
        assert!(extracted.svgs[0].contains(r#"width="1280px" height="720px""#));

        let pdf = compile_svgs_to_pdf(&extracted.svgs, &super::super::noop_progress(), "test")
            .expect("should compile valid 16:9 PDF");
        let pdf_str = String::from_utf8_lossy(&pdf);
        assert!(pdf_str.starts_with("%PDF-1.4"));
        assert!(pdf_str.contains("/Count 2"));
        assert!(pdf_str.contains("/MediaBox [0 0 1280 720]"));
    }

    #[test]
    fn natural_slide_number_sorting() {
        assert_eq!(extraer_numero_slide("diapositiva_1.svg"), 1);
        assert_eq!(extraer_numero_slide("diapositiva_10.svg"), 10);
        assert_eq!(extraer_numero_slide("slide_2.svg"), 2);
    }
}
