use clap::Parser;

mod commands;
mod output;
mod reporter;

#[derive(Parser)]
#[command(name = "omniget", version, about = "Download media from 1800+ sites", long_about = None)]
struct Cli {
    #[arg(long, global = true, help = "Output in JSON format")]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Download a video/audio from URL
    Download {
        url: String,

        #[arg(short, long, help = "Video quality height (e.g. 720, 1080)")]
        quality: Option<u32>,

        #[arg(short, long, help = "Output directory")]
        output: Option<String>,

        #[arg(long, help = "Download audio only")]
        audio_only: bool,

        #[arg(long, help = "Subtitle languages (e.g. en,zh-Hans)")]
        subs: Option<String>,

        #[arg(long, help = "Format preference (mp4/mkv/webm)")]
        format: Option<String>,
    },
    /// Preview media info without downloading
    Info {
        url: String,
    },
    /// Batch download from a file (one URL per line)
    Batch {
        file: String,

        #[arg(short, long, default_value = "3")]
        max_concurrent: usize,

        #[arg(short, long)]
        output: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "omniget_cli=info".into()),
        )
        .init();

    let cli = Cli::parse();

    if cli.json {
        output::set_json_mode(true);
    }

    match cli.command {
        Commands::Download {
            url,
            quality,
            output,
            audio_only,
            subs,
            format,
        } => {
            commands::download::execute(url, quality, output, audio_only, subs, format).await?;
        }
        Commands::Info { url } => {
            commands::info::execute(url).await?;
        }
        Commands::Batch {
            file,
            max_concurrent,
            output,
        } => {
            commands::batch::execute(file, max_concurrent, output).await?;
        }
    }

    Ok(())
}
