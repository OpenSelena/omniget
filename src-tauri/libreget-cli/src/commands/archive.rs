//! `libreget archive`: acervo retomável numa pasta, pela linha de comando.
//!
//! Embrulha `yt_archive` (o mesmo motor da tela "Arquivar" do app) e o
//! `yt_dataset` (semeadura a partir de buscas). Rodar de novo continua de onde
//! parou; Ctrl+C devolve o item em andamento à fila. Com `--json` cada evento
//! sai numa linha, para um agente acompanhar sem interpretar texto.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use libreget_core::core::tools::yt_archive::{self, ArchiveResult, ItemStatus, Options};
use libreget_core::core::tools::{yt_dataset, ProgressFn, ToolProgress};
use serde_json::json;

use crate::commands::common;
use crate::output;

#[derive(clap::Args)]
pub struct ArchiveArgs {
    /// Channel, playlist or video URL to enumerate into the queue (optional with --seed)
    pub url: Option<String>,

    #[arg(long, help = "Archive folder (the queue state lives in it)")]
    pub dest: String,

    #[arg(
        long,
        help = "NDJSON from `libreget search --json` to add to the queue"
    )]
    pub seed: Option<String>,

    #[arg(
        short,
        long,
        default_value = "best",
        help = "best | audio | max height (e.g. 360)"
    )]
    pub quality: String,

    #[arg(long, help = "Subtitle languages, yt-dlp syntax (e.g. \".*-orig,en\")")]
    pub subs: Option<String>,

    #[arg(long, help = "Save the .info.json metadata next to each video")]
    pub info_json: bool,

    #[arg(long, help = "Save the thumbnail next to each video")]
    pub thumbnail: bool,

    #[arg(
        long,
        default_value_t = 0,
        help = "Stop after N items this run (0 = all)"
    )]
    pub max_items: u32,

    #[arg(long, help = "Stop when the folder reaches this size in GB")]
    pub max_gb: Option<f64>,

    #[arg(
        long,
        default_value_t = 0,
        help = "Seconds to wait before each download"
    )]
    pub sleep: u32,

    #[arg(long, help = "Put failed items back in the queue")]
    pub retry_failed: bool,

    #[arg(long, help = "cookies.txt (Netscape) to send with the requests")]
    pub cookies: Option<String>,

    #[arg(long, help = "Only build the queue; download nothing")]
    pub scan_only: bool,
}

pub async fn execute(args: ArchiveArgs, proxy: Option<String>) -> Result<()> {
    common::init_cli_runtime(proxy.as_deref())?;
    let dest = PathBuf::from(&args.dest);
    std::fs::create_dir_all(&dest).with_context(|| format!("creating {}", dest.display()))?;
    if let Some(seed) = &args.seed {
        seed_queue(&dest, seed)?;
    }
    let opts = build_options(&args)?;
    let has_queue = yt_archive::load_state(&dest).is_some();
    if opts.url.is_empty() && !has_queue {
        bail!("nothing to archive: pass a URL or --seed");
    }

    let result = if args.scan_only {
        if opts.url.is_empty() {
            yt_archive::state(&args.dest)?
        } else {
            yt_archive::scan(&opts, &progress_printer()).await?
        }
    } else {
        tokio::spawn(async {
            if tokio::signal::ctrl_c().await.is_ok() {
                eprintln!("stopping: the current item goes back to the queue");
                yt_archive::cancel();
            }
        });
        yt_archive::run(opts, progress_printer()).await?
    };
    print_summary(&result, &dest);
    Ok(())
}

fn seed_queue(dest: &Path, seed: &str) -> Result<()> {
    let text = std::fs::read_to_string(seed).with_context(|| format!("reading {seed}"))?;
    let title = dest
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let r = yt_dataset::seed(dest, &title, yt_dataset::parse_seed(&text))?;
    if output::is_json_mode() {
        println!(
            "{}",
            json!({ "type": "seeded", "added": r.added, "total": r.total })
        );
    } else {
        println!("Queue: +{} new, {} total", r.added, r.total);
    }
    Ok(())
}

/// Com URL, a coleção é sempre reenumerada e fundida à fila: rodar de novo
/// pega os vídeos novos do canal sem perder o que já foi baixado.
fn build_options(args: &ArchiveArgs) -> Result<Options> {
    let session_netscape = match &args.cookies {
        Some(path) => {
            Some(std::fs::read_to_string(path).with_context(|| format!("reading {path}"))?)
        }
        None => None,
    };
    let url = args.url.clone().unwrap_or_default().trim().to_string();
    Ok(Options {
        refresh: !url.is_empty(),
        url,
        dest: args.dest.clone(),
        quality: args.quality.clone(),
        write_subs: args.subs.is_some(),
        sub_langs: args.subs.clone(),
        write_thumbnail: args.thumbnail,
        write_info_json: args.info_json,
        max_items: args.max_items,
        retry_failed: args.retry_failed,
        account_slug: None,
        session_netscape,
        max_bytes: args.max_gb.map(gb_to_bytes).unwrap_or(0),
        sleep_seconds: args.sleep,
    })
}

fn gb_to_bytes(gb: f64) -> u64 {
    (gb.max(0.0) * 1_000_000_000.0) as u64
}

/// O percentual de cada item ("item") é ruidoso demais para um log; só a
/// troca de item e o fim são impressos.
fn progress_printer() -> ProgressFn {
    Arc::new(|p: ToolProgress| {
        if p.stage == "item" {
            return;
        }
        if output::is_json_mode() {
            println!(
                "{}",
                json!({ "type": "progress", "stage": p.stage, "done": p.done,
                        "total": p.total, "message": p.message })
            );
        } else if let Some(msg) = p.message {
            println!("[{}/{}] {}", p.done, p.total.unwrap_or(0), msg);
        }
    })
}

fn print_summary(r: &ArchiveResult, dest: &Path) {
    let failures: Vec<_> = r
        .items
        .iter()
        .filter(|i| i.status == ItemStatus::Failed)
        .take(10)
        .map(|i| json!({ "id": i.id, "title": i.title, "error": i.error }))
        .collect();
    let size_bytes = yt_dataset::dir_size(dest);
    if output::is_json_mode() {
        println!(
            "{}",
            json!({ "type": "summary", "dest": r.dest, "state_path": r.state_path,
                    "total": r.total, "done": r.done, "failed": r.failed,
                    "skipped": r.skipped, "pending": r.pending,
                    "cap_reached": r.cap_reached, "cancelled": r.cancelled,
                    "blocked": r.blocked,
                    "size_bytes": size_bytes, "failures": failures })
        );
        return;
    }
    println!(
        "Done {} | failed {} | skipped {} | pending {} | total {} | {:.2} GB",
        r.done,
        r.failed,
        r.skipped,
        r.pending,
        r.total,
        size_bytes as f64 / 1e9
    );
    if r.cap_reached {
        println!("Stopped: the folder reached the --max-gb cap");
    }
    if r.cancelled {
        println!("Stopped by request; run again to continue");
    }
    if r.blocked {
        println!("Stopped: YouTube kept flagging this IP as a bot; run again in a few hours");
    }
    for f in &failures {
        println!("  failed {} — {}", f["id"], f["error"]);
    }
}
