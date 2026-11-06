//! `omniget search`: busca por texto e imprime um vídeo por linha.
//!
//! Com `--json` a saída é NDJSON e serve de semente para `omniget archive
//! --seed`, o que permite a um agente montar um acervo a partir de uma lista
//! de termos sem abrir o app. Progresso e erros vão para o stderr para não
//! sujar o NDJSON do stdout.

use std::collections::HashSet;

use anyhow::{bail, Context, Result};
use omniget_core::core::tools::yt_search::{self, SearchHit, SearchOptions};

use crate::commands::common;
use crate::output;

#[derive(clap::Args)]
pub struct SearchArgs {
    /// Search terms (each one is a separate search)
    pub queries: Vec<String>,

    #[arg(long, help = "File with one search term per line (# starts a comment)")]
    pub queries_file: Option<String>,

    #[arg(
        short = 'n',
        long,
        default_value_t = 20,
        help = "Results per term (max 500)"
    )]
    pub max_results: u32,

    #[arg(long, default_value = "youtube", help = "youtube | soundcloud")]
    pub platform: String,

    #[arg(long, help = "Drop videos shorter than this (seconds)")]
    pub min_duration: Option<f64>,

    #[arg(long, help = "Drop videos longer than this (seconds)")]
    pub max_duration: Option<f64>,

    #[arg(
        long,
        value_delimiter = ',',
        help = "Keep titles containing any of these (comma-separated)"
    )]
    pub title_has: Vec<String>,
}

pub async fn execute(args: SearchArgs, proxy: Option<String>) -> Result<()> {
    common::init_cli_runtime(proxy.as_deref())?;
    let queries = collect_queries(&args)?;
    if queries.is_empty() {
        bail!("give at least one search term or --queries-file");
    }

    let mut seen = HashSet::new();
    let mut failures = 0usize;
    for query in &queries {
        let opts = SearchOptions {
            query: query.clone(),
            platform: args.platform.clone(),
            max_results: args.max_results,
            min_duration: args.min_duration,
            max_duration: args.max_duration,
            title_has: args.title_has.clone(),
        };
        match yt_search::search(&opts).await {
            Ok(hits) => {
                let fresh: Vec<SearchHit> = hits
                    .into_iter()
                    .filter(|h| seen.insert(h.id.clone()))
                    .collect();
                eprintln!("{query}: {} new", fresh.len());
                fresh.iter().try_for_each(print_hit)?;
            }
            Err(e) => {
                failures += 1;
                eprintln!("{query}: {e}");
            }
        }
    }
    if failures == queries.len() {
        bail!("every search failed");
    }
    Ok(())
}

fn collect_queries(args: &SearchArgs) -> Result<Vec<String>> {
    let mut out: Vec<String> = args.queries.clone();
    if let Some(path) = &args.queries_file {
        let text = std::fs::read_to_string(path).with_context(|| format!("reading {path}"))?;
        out.extend(
            text.lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .map(str::to_string),
        );
    }
    Ok(out)
}

fn print_hit(hit: &SearchHit) -> Result<()> {
    if output::is_json_mode() {
        println!("{}", serde_json::to_string(hit)?);
    } else {
        let mins = hit.duration.map(|d| format!("{:>4.0}m", d / 60.0));
        println!(
            "{}  {:<24.24}  {}  {}",
            mins.unwrap_or_else(|| "    ?".into()),
            hit.uploader,
            hit.title,
            hit.url
        );
    }
    Ok(())
}
