use crate::client::BROWSER_UA;
use crate::error::{Error, Result};
use std::path::Path;

/// Download an HLS playlist to MP4 using bundled ffmpeg.
pub fn download_hls(url: &str, output: &Path, referer: Option<&str>) -> Result<()> {
    ffmpeg_sidecar::download::auto_download().map_err(|e| Error::Merge(e.to_string()))?;

    let mut headers = format!("User-Agent: {BROWSER_UA}\r\n");
    if let Some(referer) = referer {
        headers.push_str(&format!("Referer: {referer}\r\n"));
    }

    let status = ffmpeg_sidecar::command::FfmpegCommand::new()
        .overwrite()
        .args(["-headers", &headers])
        .input(url)
        .output(output.to_string_lossy().as_ref())
        .args(["-c", "copy", "-movflags", "+faststart"])
        .spawn()
        .map_err(|e| Error::Merge(e.to_string()))?
        .wait()
        .map_err(|e| Error::Merge(e.to_string()))?;

    if !status.success() {
        return Err(Error::Merge("ffmpeg HLS download failed".into()));
    }
    Ok(())
}

/// Mux video + audio into one MP4 using a bundled ffmpeg binary (`ffmpeg-sidecar`).
pub fn mux_copy(video: &Path, audio: &Path, output: &Path) -> Result<()> {
    ffmpeg_sidecar::download::auto_download().map_err(|e| Error::Merge(e.to_string()))?;

    let status = ffmpeg_sidecar::command::FfmpegCommand::new()
        .overwrite()
        .input(video.to_string_lossy().as_ref())
        .input(audio.to_string_lossy().as_ref())
        .output(output.to_string_lossy().as_ref())
        .args(["-c", "copy", "-movflags", "+faststart"])
        .spawn()
        .map_err(|e| Error::Merge(e.to_string()))?
        .wait()
        .map_err(|e| Error::Merge(e.to_string()))?;

    if !status.success() {
        return Err(Error::Merge("ffmpeg exited with an error".into()));
    }
    Ok(())
}

/// Warm the ffmpeg cache (optional; `mux_copy` also downloads on first use).
pub fn prefetch_tools() -> Result<()> {
    ffmpeg_sidecar::download::auto_download().map_err(|e| Error::Merge(e.to_string()))?;
    Ok(())
}
