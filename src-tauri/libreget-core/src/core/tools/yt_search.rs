//! Busca de vídeos por texto (YouTube/SoundCloud) via `ytsearchN:`.
//!
//! Mora no core, e não só no comando Tauri, para que a UI e o `libreget-cli`
//! usem a mesma busca: a CLI precisa dela para montar acervos a partir de uma
//! lista de termos (ex.: `libreget search "mir4 boss" --json > hits.ndjson`),
//! e os filtros de duração/título ficam numa função pura testável sem rede.

use serde::Serialize;

/// Teto do yt-dlp por consulta; acima disso a busca fica lenta e o YouTube
/// começa a devolver páginas vazias.
pub const MAX_RESULTS: u32 = 500;

#[derive(Debug, Clone, Default)]
pub struct SearchOptions {
    pub query: String,
    /// "youtube" (padrão) | "soundcloud"
    pub platform: String,
    pub max_results: u32,
    /// Descarta vídeos mais curtos que isso (segundos).
    pub min_duration: Option<f64>,
    /// Descarta vídeos mais longos que isso (segundos).
    pub max_duration: Option<f64>,
    /// Mantém só títulos que contenham algum destes termos (sem caixa).
    pub title_has: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SearchHit {
    pub id: String,
    pub title: String,
    pub url: String,
    pub uploader: String,
    pub duration: Option<f64>,
    pub view_count: Option<u64>,
    pub upload_date: Option<String>,
    pub thumbnail_url: Option<String>,
    pub platform: String,
    /// Termo que trouxe o vídeo — vira metadado de proveniência no acervo.
    pub query: String,
}

fn search_prefix(platform: &str) -> &'static str {
    match platform {
        "soundcloud" => "scsearch",
        _ => "ytsearch",
    }
}

/// Roda a busca no yt-dlp e devolve os resultados já filtrados.
pub async fn search(opts: &SearchOptions) -> anyhow::Result<Vec<SearchHit>> {
    let query = opts.query.trim();
    if query.is_empty() {
        anyhow::bail!("search query is empty");
    }
    let ytdlp_path = crate::core::ytdlp::ensure_ytdlp()
        .await
        .map_err(|e| anyhow::anyhow!("yt-dlp unavailable: {}", e))?;
    let n = opts.max_results.clamp(1, MAX_RESULTS);

    let mut args = vec![
        "--flat-playlist".to_string(),
        "--dump-json".to_string(),
        "--no-warnings".to_string(),
        "--socket-timeout".to_string(),
        "15".to_string(),
    ];
    args.extend(crate::core::ytdlp::insecure_tls_args());
    args.push(format!("{}{}:{}", search_prefix(&opts.platform), n, query));

    let output = crate::core::ytdlp::ytdlp_command(&ytdlp_path)
        .args(&args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to run yt-dlp: {}", e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Search failed: {}", stderr.trim());
    }
    Ok(parse_hits(&String::from_utf8_lossy(&output.stdout), opts))
}

fn str_field(json: &serde_json::Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .filter_map(|k| json.get(*k).and_then(|v| v.as_str()))
        .find(|s| !s.is_empty())
        .map(str::to_string)
}

fn thumbnail_of(json: &serde_json::Value) -> Option<String> {
    json.get("thumbnails")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.last())
        .and_then(|t| t.get("url"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .or_else(|| str_field(json, &["thumbnail"]))
}

fn hit_from(json: &serde_json::Value, opts: &SearchOptions) -> Option<SearchHit> {
    let id = str_field(json, &["id"])?;
    let url = str_field(json, &["url", "webpage_url"])
        .unwrap_or_else(|| format!("https://www.youtube.com/watch?v={}", id));
    Some(SearchHit {
        title: str_field(json, &["title"]).unwrap_or_else(|| "Untitled".into()),
        uploader: str_field(json, &["uploader", "channel", "uploader_id"]).unwrap_or_default(),
        duration: json.get("duration").and_then(|v| v.as_f64()),
        view_count: json.get("view_count").and_then(|v| v.as_u64()),
        upload_date: str_field(json, &["upload_date"]),
        thumbnail_url: thumbnail_of(json),
        platform: if opts.platform.is_empty() {
            "youtube".into()
        } else {
            opts.platform.clone()
        },
        query: opts.query.trim().to_string(),
        id,
        url,
    })
}

/// Sem duração conhecida (live, estreia) o vídeo só passa quando nenhum
/// limite foi pedido: uma live não termina e estouraria o acervo.
fn passes(hit: &SearchHit, opts: &SearchOptions, needles: &[String]) -> bool {
    if opts.min_duration.is_some() || opts.max_duration.is_some() {
        let Some(d) = hit.duration else { return false };
        if opts.min_duration.is_some_and(|min| d < min) {
            return false;
        }
        if opts.max_duration.is_some_and(|max| d > max) {
            return false;
        }
    }
    if needles.is_empty() {
        return true;
    }
    let title = hit.title.to_lowercase();
    needles.iter().any(|n| title.contains(n.as_str()))
}

/// Lê o NDJSON do `--dump-json`, aplica os filtros e tira repetidos.
pub fn parse_hits(stdout: &str, opts: &SearchOptions) -> Vec<SearchHit> {
    let needles: Vec<String> = opts
        .title_has
        .iter()
        .map(|t| t.trim().to_lowercase())
        .filter(|t| !t.is_empty())
        .collect();
    let mut seen = std::collections::HashSet::new();
    stdout
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line.trim()).ok())
        .filter_map(|json| hit_from(&json, opts))
        .filter(|hit| passes(hit, opts, &needles))
        .filter(|hit| seen.insert(hit.id.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const OUT: &str = r#"{"id":"a1","title":"MIR4 Boss Guide","url":"https://www.youtube.com/watch?v=a1","channel":"Chan","duration":600.0,"view_count":10}
{"id":"b2","title":"Minecraft speedrun","duration":900.0}

not json
{"id":"c3","title":"미르4 공략","duration":60.0}
{"id":"d4","title":"mir4 live","duration":null}
{"id":"a1","title":"MIR4 Boss Guide","duration":600.0}
{"title":"sem id"}"#;

    fn opts() -> SearchOptions {
        SearchOptions {
            query: " mir4 ".into(),
            platform: "youtube".into(),
            max_results: 10,
            ..Default::default()
        }
    }

    #[test]
    fn parses_every_hit_once_without_filters() {
        let hits = parse_hits(OUT, &opts());
        let ids: Vec<&str> = hits.iter().map(|h| h.id.as_str()).collect();
        assert_eq!(ids, ["a1", "b2", "c3", "d4"], "repetido e sem id saem");
        assert_eq!(hits[0].uploader, "Chan");
        assert_eq!(hits[0].view_count, Some(10));
        assert_eq!(hits[0].query, "mir4");
        assert_eq!(hits[1].url, "https://www.youtube.com/watch?v=b2");
    }

    #[test]
    fn duration_filters_drop_short_long_and_unknown() {
        let o = SearchOptions {
            min_duration: Some(120.0),
            max_duration: Some(700.0),
            ..opts()
        };
        let ids: Vec<String> = parse_hits(OUT, &o).into_iter().map(|h| h.id).collect();
        assert_eq!(ids, ["a1"], "60s é curto, 900s longo, live sem duração");
    }

    #[test]
    fn title_filter_is_case_insensitive_and_any_match() {
        let o = SearchOptions {
            title_has: vec!["MIR4".into(), "미르4".into(), " ".into()],
            ..opts()
        };
        let ids: Vec<String> = parse_hits(OUT, &o).into_iter().map(|h| h.id).collect();
        assert_eq!(ids, ["a1", "c3", "d4"]);
    }

    #[test]
    fn soundcloud_uses_its_own_prefix() {
        assert_eq!(search_prefix("soundcloud"), "scsearch");
        assert_eq!(search_prefix(""), "ytsearch");
    }

    /// Precisa de rede e do yt-dlp.
    /// `cargo test -p libreget-core --lib -- --ignored live_yt_search`
    #[tokio::test]
    #[ignore]
    async fn live_yt_search_returns_hits() {
        let o = SearchOptions {
            query: "mir4 gameplay".into(),
            max_results: 3,
            ..opts()
        };
        let hits = search(&o).await.expect("a busca devia responder");
        assert!(!hits.is_empty());
        eprintln!("{:?}", hits.iter().map(|h| &h.title).collect::<Vec<_>>());
    }
}
