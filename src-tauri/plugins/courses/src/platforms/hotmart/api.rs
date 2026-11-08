use anyhow::anyhow;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

use super::auth::HotmartSession;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Course {
    pub id: u64,
    pub name: String,
    pub slug: Option<String>,
    pub seller: String,
    pub subdomain: Option<String>,
    pub is_hotmart_club: bool,
    pub price: Option<f64>,
    pub image_url: Option<String>,
    pub category: Option<String>,
    pub external_platform: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Module {
    pub id: String,
    pub name: String,
    pub pages: Vec<PageInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageInfo {
    pub hash: String,
    pub name: String,
    pub page_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lesson {
    pub hash: String,
    pub name: String,
    pub content: Option<String>,
    pub has_media: bool,
    pub medias: Vec<LessonMedia>,
    pub attachments: Vec<Attachment>,
    pub complementary_readings: Option<Vec<ReadingLink>>,
    pub locked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LessonMedia {
    pub name: String,
    pub code: String,
    pub url: String,
    pub media_type: String,
    pub size: Option<u64>,
    pub duration: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub file_membership_id: String,
    pub file_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadingLink {
    pub title: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentInfo {
    pub url: String,
    pub file_name: Option<String>,
    pub token: Option<String>,
    pub lambda_url: Option<String>,
    pub is_drm: bool,
}

#[derive(Debug, Clone)]
pub struct SubdomainInfo {
    pub product_id: u64,
    pub subdomain: String,
}

pub fn navigation_headers(token: &str, slug: &str, product_id: u64) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Accept",
        HeaderValue::from_static("application/json, text/plain, */*"),
    );
    headers.insert(
        "Authorization",
        format!("Bearer {}", token).parse().unwrap(),
    );
    headers.insert("Origin", HeaderValue::from_static("https://hotmart.com"));
    headers.insert("Referer", HeaderValue::from_static("https://hotmart.com"));
    headers.insert("Pragma", HeaderValue::from_static("no-cache"));
    headers.insert("cache-control", HeaderValue::from_static("no-cache"));
    headers.insert("slug", slug.parse().unwrap());
    headers.insert("x-product-id", product_id.to_string().parse().unwrap());
    headers
}

pub async fn get_subdomains(session: &HotmartSession) -> anyhow::Result<Vec<SubdomainInfo>> {
    let resp = session
        .client
        .post("https://api-sec-vlc.hotmart.com/security/oauth/check_token")
        .form(&[("token", &session.token)])
        .send()
        .await?;

    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "check_token retornou status {}: {}",
            status,
            &body_text[..500.min(body_text.len())]
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;

    let resources = body
        .get("resources")
        .and_then(|r| r.as_array())
        .ok_or_else(|| anyhow!("Field 'resources' not found in check_token"))?;

    let mut subdomains = Vec::new();
    for res in resources {
        let resource = match res.get("resource") {
            Some(r) => r,
            None => continue,
        };
        let product_id = resource
            .get("productId")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let subdomain = resource
            .get("subdomain")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if product_id > 0 && !subdomain.is_empty() {
            subdomains.push(SubdomainInfo {
                product_id,
                subdomain,
            });
        }
    }

    Ok(subdomains)
}

pub async fn list_courses(session: &HotmartSession) -> anyhow::Result<Vec<Course>> {
    let resp = session
        .client
        .get("https://api-hub.cb.hotmart.com/club-drive-api/rest/v2/purchase/?archived=UNARCHIVED")
        .send()
        .await?;

    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "list_courses retornou status {}: {}",
            status,
            &body_text[..500.min(body_text.len())]
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;

    let purchases = body
        .get("data")
        .and_then(|d| d.as_array())
        .or_else(|| body.get("purchases").and_then(|p| p.as_array()))
        .or_else(|| body.as_array())
        .ok_or_else(|| anyhow!("Formato inesperado na resposta de cursos: sem campo 'data', 'purchases' ou array raiz"))?;

    let mut courses = Vec::new();
    for p in purchases {
        let product = p.get("product").unwrap_or(p);

        let id = product.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
        let name = product
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let seller = product
            .get("seller")
            .and_then(|s| s.get("name").and_then(|n| n.as_str()))
            .or_else(|| {
                p.get("producer")
                    .or_else(|| p.get("seller"))
                    .and_then(|s| s.get("name").and_then(|n| n.as_str()))
            })
            .unwrap_or("")
            .to_string();

        let slug = product
            .get("hotmartClub")
            .and_then(|hc| hc.get("slug").and_then(|s| s.as_str()))
            .map(String::from);

        let is_hotmart_club = slug.is_some()
            || p.get("accessRights")
                .and_then(|a| a.get("hasClubAccess"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

        let category = product
            .get("category")
            .and_then(|v| v.as_str())
            .map(String::from);
        let image_url = product
            .get("picture")
            .and_then(|v| v.as_str())
            .map(String::from);

        courses.push(Course {
            id,
            name,
            slug,
            seller,
            subdomain: None,
            is_hotmart_club,
            price: None,
            image_url,
            category,
            external_platform: false,
        });
    }

    Ok(courses)
}

pub async fn get_course_price(session: &HotmartSession, product_id: u64) -> anyhow::Result<f64> {
    let url = format!(
        "https://api-hub.cb.hotmart.com/club-drive-api/rest/v2/purchase/products/{}",
        product_id
    );

    let resp = session.client.get(&url).send().await?;

    let status = resp.status();
    if !status.is_success() {
        return Err(anyhow!("Price not available (status {})", status));
    }

    let body: serde_json::Value = resp.json().await?;

    let price = body
        .get("purchases")
        .and_then(|p| p.as_array())
        .and_then(|arr| arr.first())
        .and_then(|purchase| purchase.get("value"))
        .and_then(|v| v.as_f64())
        .or_else(|| body.get("value").and_then(|v| v.as_f64()))
        .unwrap_or(0.0);

    Ok(price)
}

pub fn merge_subdomains(courses: &mut [Course], subdomains: &[SubdomainInfo]) {
    for course in courses.iter_mut() {
        if let Some(info) = subdomains.iter().find(|s| s.product_id == course.id) {
            course.subdomain = Some(info.subdomain.clone());
            if course.slug.is_none() {
                course.slug = Some(info.subdomain.clone());
            }
        }

        if course.slug.is_none() && course.subdomain.is_none() {
            course.external_platform = true;
        }
    }
}

pub async fn get_modules(
    session: &HotmartSession,
    slug: &str,
    product_id: u64,
) -> anyhow::Result<Vec<Module>> {
    let resp = session
        .client
        .get("https://api-club-course-consumption-gateway-ga.cb.hotmart.com/v1/navigation")
        .headers(navigation_headers(&session.token, slug, product_id))
        .send()
        .await?;

    let status = resp.status();
    let body_text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "get_modules retornou status {}: {}",
            status,
            &body_text[..500.min(body_text.len())]
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)?;

    let modules_json = body
        .get("modules")
        .and_then(|m| m.as_array())
        .or_else(|| body.as_array())
        .ok_or_else(|| anyhow!("Unexpected format in modules response"))?;

    let mut modules = Vec::new();
    for m in modules_json {
        let id = m
            .get("id")
            .map(|v| match v {
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::String(s) => s.clone(),
                _ => String::new(),
            })
            .unwrap_or_default();
        let name = m
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let pages_json = m
            .get("pages")
            .and_then(|p| p.as_array())
            .cloned()
            .unwrap_or_default();

        let pages = pages_json
            .iter()
            .map(|p| PageInfo {
                hash: p
                    .get("hash")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                name: p
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                page_type: p
                    .get("type")
                    .or_else(|| p.get("pageType"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
            })
            .collect();

        modules.push(Module { id, name, pages });
    }

    Ok(modules)
}

pub fn parse_attachments(body: &serde_json::Value) -> Vec<Attachment> {
    body.get("attachments")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|a| {
                    let file_membership_id = a
                        .get("fileMembershipId")
                        .or_else(|| a.get("id"))
                        .map(|v| match v {
                            serde_json::Value::Number(n) => n.to_string(),
                            serde_json::Value::String(s) => s.trim().to_string(),
                            _ => String::new(),
                        })
                        .unwrap_or_default();
                    if file_membership_id.is_empty() {
                        return None;
                    }
                    let file_name = a
                        .get("fileName")
                        .or_else(|| a.get("name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    Some(Attachment {
                        file_membership_id,
                        file_name,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn parse_complementary_readings(body: &serde_json::Value) -> Option<Vec<ReadingLink>> {
    let readings: Vec<ReadingLink> = body
        .get("complementaryReadings")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|r| {
                    let title = r
                        .get("articleName")
                        .or_else(|| r.get("title"))
                        .and_then(|v| v.as_str())
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(String::from);
                    let url = r
                        .get("articleUrl")
                        .or_else(|| r.get("url"))
                        .and_then(|v| v.as_str())
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(String::from);
                    if title.is_none() && url.is_none() {
                        return None;
                    }
                    Some(ReadingLink { title, url })
                })
                .collect()
        })
        .unwrap_or_default();

    if readings.is_empty() {
        None
    } else {
        Some(readings)
    }
}

pub async fn get_complementary_content(
    session: &HotmartSession,
    slug: &str,
    product_id: u64,
    page_hash: &str,
) -> anyhow::Result<(Vec<Attachment>, Option<Vec<ReadingLink>>)> {
    let url = format!(
        "https://api-club-course-consumption-gateway-ga.cb.hotmart.com/v1/pages/{}/complementary-content",
        page_hash
    );
    let resp = session
        .client
        .get(&url)
        .headers(navigation_headers(&session.token, slug, product_id))
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        let body_text = resp.text().await.unwrap_or_default();
        return Err(anyhow!(
            "get_complementary_content retornou status {}: {}",
            status,
            &body_text[..500.min(body_text.len())]
        ));
    }

    let body: serde_json::Value = resp.json().await?;
    Ok((
        parse_attachments(&body),
        parse_complementary_readings(&body),
    ))
}

pub async fn get_lesson(
    session: &HotmartSession,
    slug: &str,
    product_id: u64,
    page_hash: &str,
) -> anyhow::Result<Lesson> {
    let url = format!(
        "https://api-club-course-consumption-gateway-ga.cb.hotmart.com/v2/web/lessons/{}",
        page_hash
    );
    let resp = session
        .client
        .get(&url)
        .headers(navigation_headers(&session.token, slug, product_id))
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        let body_text = resp.text().await.unwrap_or_default();
        return Err(anyhow!(
            "get_lesson retornou status {}: {}",
            status,
            &body_text[..500.min(body_text.len())]
        ));
    }

    let body: serde_json::Value = resp.json().await?;

    if let Some(msg) = body.get("message").and_then(|v| v.as_str()) {
        return Err(anyhow!("Lesson unavailable: {}", msg));
    }

    let hash = body
        .get("hash")
        .and_then(|v| v.as_str())
        .unwrap_or(page_hash)
        .to_string();
    let name = body
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let content = body
        .get("content")
        .and_then(|v| v.as_str())
        .map(String::from);
    let locked = body
        .get("locked")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let has_media = body
        .get("hasMedia")
        .or_else(|| body.get("hasPlayerMedia"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let medias_arr = body
        .get("medias")
        .or_else(|| body.get("mediasSrc"))
        .and_then(|v| v.as_array());

    let medias: Vec<LessonMedia> = medias_arr
        .map(|arr| {
            arr.iter()
                .map(|m| LessonMedia {
                    name: m
                        .get("name")
                        .or_else(|| m.get("mediaName"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    code: m
                        .get("code")
                        .or_else(|| m.get("mediaCode"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    url: m
                        .get("url")
                        .or_else(|| m.get("mediaSrcUrl"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    media_type: m
                        .get("type")
                        .or_else(|| m.get("mediaType"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    size: m.get("size").and_then(|v| v.as_u64()),
                    duration: m.get("duration").and_then(|v| v.as_u64()),
                })
                .collect()
        })
        .unwrap_or_default();

    let mut attachments = parse_attachments(&body);
    let mut complementary_readings = parse_complementary_readings(&body);

    if attachments.is_empty() && complementary_readings.is_none() {
        match get_complementary_content(session, slug, product_id, page_hash).await {
            Ok((comp_attachments, comp_readings)) => {
                attachments = comp_attachments;
                complementary_readings = comp_readings;
            }
            Err(e) => {
                tracing::warn!(
                    "[hotmart] falha ao buscar complementary-content da aula '{}': {}",
                    page_hash,
                    e
                );
            }
        }
    }

    Ok(Lesson {
        hash,
        name,
        content,
        has_media,
        medias,
        attachments,
        complementary_readings,
        locked,
    })
}

pub fn parse_attachment_info(body_text: &str) -> anyhow::Result<AttachmentInfo> {
    let trimmed = body_text.trim();
    let unquoted = trimmed.trim_matches('"').trim();
    if unquoted.starts_with("http://") || unquoted.starts_with("https://") {
        return Ok(AttachmentInfo {
            url: unquoted.to_string(),
            file_name: None,
            token: None,
            lambda_url: None,
            is_drm: false,
        });
    }

    let body: serde_json::Value = serde_json::from_str(trimmed)?;
    let is_drm = trimmed.contains("drm-protection");

    let download_url = body
        .get("directDownloadUrl")
        .or_else(|| body.get("url"))
        .or_else(|| body.get("downloadUrl"))
        .or_else(|| body.get("signedUrl"))
        .or_else(|| body.get("link"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let file_name = body
        .get("fileName")
        .and_then(|v| v.as_str())
        .map(String::from);
    let token = body.get("token").and_then(|v| v.as_str()).map(String::from);
    let lambda_url = body
        .get("lambdaUrl")
        .and_then(|v| v.as_str())
        .map(String::from);

    Ok(AttachmentInfo {
        url: download_url,
        file_name,
        token,
        lambda_url,
        is_drm,
    })
}

pub async fn get_attachment_url(
    session: &HotmartSession,
    slug: &str,
    product_id: u64,
    id: &str,
) -> anyhow::Result<AttachmentInfo> {
    let url = format!(
        "https://api-club-hot-club-api.cb.hotmart.com/rest/v3/attachment/{}/download",
        id
    );
    let resp = session
        .client
        .get(&url)
        .headers(navigation_headers(&session.token, slug, product_id))
        .send()
        .await?;

    let status = resp.status();
    let body_text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(anyhow!(
            "get_attachment_url retornou status {} para {}: {}",
            status,
            id,
            &body_text[..500.min(body_text.len())]
        ));
    }

    parse_attachment_info(&body_text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_complementary_content_attachments_and_readings() {
        let raw = r#"{
            "complementaryReadings": [
                {
                    "articleName": "Guia de Referencia",
                    "articleUrl": "https://example.com/guia"
                }
            ],
            "attachments": [
                {
                    "fileOrder": 4,
                    "fileMembershipId": "641db8a0-b918-4460-b007-a001b9f79bb5",
                    "fileName": "PDF DIAPOSITIVAS MMSS_watermark (1).pdf",
                    "fileSize": 83952102
                },
                {
                    "fileOrder": 5,
                    "fileMembershipId": 987654,
                    "fileName": "Apostila.pdf"
                }
            ]
        }"#;
        let json: serde_json::Value = serde_json::from_str(raw).unwrap();
        let attachments = parse_attachments(&json);
        assert_eq!(attachments.len(), 2);
        assert_eq!(
            attachments[0].file_membership_id,
            "641db8a0-b918-4460-b007-a001b9f79bb5"
        );
        assert_eq!(
            attachments[0].file_name,
            "PDF DIAPOSITIVAS MMSS_watermark (1).pdf"
        );
        assert_eq!(attachments[1].file_membership_id, "987654");
        assert_eq!(attachments[1].file_name, "Apostila.pdf");

        let readings = parse_complementary_readings(&json).unwrap();
        assert_eq!(readings.len(), 1);
        assert_eq!(readings[0].title.as_deref(), Some("Guia de Referencia"));
        assert_eq!(readings[0].url.as_deref(), Some("https://example.com/guia"));
    }

    #[test]
    fn empty_complementary_content_yields_empty_attachments_and_none_readings() {
        let raw = r#"{"complementaryReadings":[],"attachments":[]}"#;
        let json: serde_json::Value = serde_json::from_str(raw).unwrap();
        assert!(parse_attachments(&json).is_empty());
        assert!(parse_complementary_readings(&json).is_none());
    }

    #[test]
    fn parses_attachment_download_info_variants() {
        let direct = r#"{"directDownloadUrl":"https://hotmart-club-files.cb.hotmart.com/membership_area/x/y.pdf?Expires=1&Signature=s"}"#;
        let info = parse_attachment_info(direct).unwrap();
        assert!(!info.is_drm);
        assert_eq!(
            info.url,
            "https://hotmart-club-files.cb.hotmart.com/membership_area/x/y.pdf?Expires=1&Signature=s"
        );

        let bare = "https://hotmart-club-files.cb.hotmart.com/membership_area/x/y.pdf";
        let info_bare = parse_attachment_info(bare).unwrap();
        assert_eq!(info_bare.url, bare);

        let drm =
            r#"{"lambdaUrl":"https://drm-protection.cb.hotmart.com/sign","token":"drm-tok-1"}"#;
        let info_drm = parse_attachment_info(drm).unwrap();
        assert!(info_drm.is_drm);
        assert_eq!(
            info_drm.lambda_url.as_deref(),
            Some("https://drm-protection.cb.hotmart.com/sign")
        );
        assert_eq!(info_drm.token.as_deref(), Some("drm-tok-1"));
    }

    #[test]
    fn navigation_headers_include_product_id_and_slug() {
        let headers = navigation_headers("jwt-tok", "anatomy-club", 6083220);
        assert_eq!(
            headers.get("Authorization").unwrap().to_str().unwrap(),
            "Bearer jwt-tok"
        );
        assert_eq!(
            headers.get("x-product-id").unwrap().to_str().unwrap(),
            "6083220"
        );
        assert_eq!(
            headers.get("slug").unwrap().to_str().unwrap(),
            "anatomy-club"
        );
    }
}
