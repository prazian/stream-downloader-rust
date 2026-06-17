//! Prefetch bundled ffmpeg into the local cache (optional; first merge also triggers download).

fn main() {
    if let Err(err) = stream_downloader::prefetch_tools() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
    println!("ffmpeg ready");
}
