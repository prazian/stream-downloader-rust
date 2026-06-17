use clap::Parser;
use std::io::Write;
use std::path::PathBuf;
use stream_downloader::{
    DownloadOptions, DownloadProgress, MediaKind, Quality, Session, format_line,
    validate_output_dir, validate_page_url,
};

#[derive(Parser, Debug)]
#[command(name = "stream-dl", about = "Download media files from web pages")]
struct Cli {
    #[arg(short = 'u', long = "url")]
    url: String,

    #[arg(short = 'o', long = "out", value_name = "DIR")]
    output: Option<PathBuf>,

    /// Video quality: best (default), 720p, 1080p, 4k, …
    #[arg(short = 'q', long = "quality", conflicts_with = "all")]
    quality: Option<String>,

    /// Download every available resolution
    #[arg(long = "all", conflicts_with = "quality")]
    all: bool,

    #[arg(long = "audio", action = clap::ArgAction::SetTrue)]
    audio: bool,
}

#[tokio::main]
async fn main() {
    if let Err(err) = run(Cli::parse()).await {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> stream_downloader::Result<()> {
    validate_page_url(&cli.url)?;
    validate_output_dir(cli.output.as_deref())?;

    let quality = if cli.all {
        Quality::All
    } else if let Some(q) = &cli.quality {
        Quality::parse(q)?
    } else {
        Quality::Best
    };

    let kinds = if cli.audio {
        vec![MediaKind::Audio]
    } else {
        vec![MediaKind::Video]
    };

    let session = Session::new();
    let on_progress = |p: DownloadProgress<'_>| {
        let _ = write!(std::io::stderr(), "\r{}", format_line(p));
        let _ = std::io::stderr().flush();
    };
    let files = session
        .download_streams(
            &cli.url,
            &kinds,
            quality,
            &DownloadOptions {
                output_dir: &cli.output.unwrap_or_else(|| PathBuf::from(".")),
                referer: None,
                on_progress: Some(&on_progress),
            },
        )
        .await?;

    let _ = writeln!(std::io::stderr());

    for file in files {
        println!("{}", file.path.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_flags_conflict() {
        assert!(Cli::try_parse_from([
            "stream-dl",
            "-u",
            "https://example.com",
            "--all",
            "-q",
            "720p"
        ])
        .is_err());
    }
}
