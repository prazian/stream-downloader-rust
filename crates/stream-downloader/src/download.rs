use crate::client;
use crate::error::Result;
use crate::merge;
use crate::model::{MediaKind, MuxPart, Stream};
use crate::naming::OutputName;
use crate::progress::DownloadProgress;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;

#[derive(Clone)]
pub struct DownloadOptions<'a> {
    pub output_dir: &'a Path,
    /// Page the stream was discovered on; sent as `Referer` for CDN hotlink checks.
    pub referer: Option<&'a str>,
    pub on_progress: Option<&'a (dyn Fn(DownloadProgress<'_>) + Send + Sync)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadedFile {
    pub stream: Stream,
    pub path: PathBuf,
}

pub struct Downloader {
    client: reqwest::Client,
}

impl Default for Downloader {
    fn default() -> Self {
        Self::new()
    }
}

impl Downloader {
    pub fn new() -> Self {
        Self::with_client(client::build())
    }

    pub fn with_client(client: reqwest::Client) -> Self {
        Self { client }
    }

    pub async fn download(
        &self,
        stream: &Stream,
        name: &OutputName,
        options: &DownloadOptions<'_>,
    ) -> Result<PathBuf> {
        tokio::fs::create_dir_all(options.output_dir).await?;
        let path = unique_path(name.path_in(options.output_dir));

        if let Some(audio) = &stream.mux_audio {
            return self
                .download_muxed(stream, audio, &path, options)
                .await;
        }

        self.download_to_path(stream, &path, options, &name.file_name)
            .await?;
        Ok(path)
    }

    async fn download_muxed(
        &self,
        video: &Stream,
        audio: &MuxPart,
        output: &Path,
        options: &DownloadOptions<'_>,
    ) -> Result<PathBuf> {
        let stem = output
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("out");
        let dir = output.parent().unwrap_or_else(|| Path::new("."));
        let video_part = dir.join(format!(".{stem}-video.part"));
        let audio_part = dir.join(format!(".{stem}-audio.part"));

        let video_label = format!("{stem} (video)");
        let audio_label = format!("{stem} (audio)");
        self.download_to_path(video, &video_part, options, &video_label)
            .await?;
        self.download_to_path(
            &Stream {
                url: audio.url.clone(),
                kind: MediaKind::Audio,
                label: None,
                download_user_agent: audio.download_user_agent,
                mux_audio: None,
            },
            &audio_part,
            options,
            &audio_label,
        )
        .await?;

        if let Some(cb) = options.on_progress {
            cb(DownloadProgress {
                label: "Muxing…",
                downloaded: 0,
                total: None,
            });
        }

        let output = output.to_path_buf();
        let v = video_part.clone();
        let a = audio_part.clone();
        let out = output.clone();
        tokio::task::spawn_blocking(move || merge::mux_copy(&v, &a, &out))
            .await
            .map_err(|e| crate::error::Error::Merge(e.to_string()))??;

        let _ = tokio::fs::remove_file(&video_part).await;
        let _ = tokio::fs::remove_file(&audio_part).await;
        Ok(output)
    }

    async fn download_to_path(
        &self,
        stream: &Stream,
        path: &Path,
        options: &DownloadOptions<'_>,
        label: &str,
    ) -> Result<()> {
        let mut request = self.client.get(stream.url.clone());
        if let Some(referer) = options.referer {
            request = request.header(reqwest::header::REFERER, referer);
        }
        if let Some(ua) = stream.download_user_agent {
            request = request.header(reqwest::header::USER_AGENT, ua);
        }

        let mut response = request.send().await?.error_for_status()?;
        let total = response.content_length();
        let mut downloaded = 0u64;
        let mut last_pct = None::<u32>;

        report(options, label, downloaded, total, &mut last_pct);

        let mut file = tokio::fs::File::create(path).await?;
        while let Some(chunk) = response.chunk().await? {
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;
            report(options, label, downloaded, total, &mut last_pct);
        }
        file.flush().await?;
        report(options, label, downloaded, total, &mut last_pct);
        Ok(())
    }
}

fn report(
    options: &DownloadOptions<'_>,
    label: &str,
    downloaded: u64,
    total: Option<u64>,
    last_pct: &mut Option<u32>,
) {
    let Some(cb) = options.on_progress else {
        return;
    };
    match total.filter(|&t| t > 0) {
        Some(t) => {
            let pct = (downloaded.saturating_mul(100) / t).min(100) as u32;
            if downloaded < t && Some(pct) == *last_pct {
                return;
            }
            *last_pct = Some(pct);
        }
        None => {
            let bucket = (downloaded / (512 * 1024)) as u32;
            if downloaded > 0 && Some(bucket) == *last_pct {
                return;
            }
            *last_pct = Some(bucket);
        }
    }
    cb(DownloadProgress {
        label,
        downloaded,
        total,
    });
}

fn unique_path(path: PathBuf) -> PathBuf {
    if !path.exists() {
        return path;
    }
    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
        return path;
    };
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let parent = path.parent().unwrap_or_else(|| Path::new("."));

    for index in 1..=999 {
        let candidate = if ext.is_empty() {
            parent.join(format!("{stem}-{index}"))
        } else {
            parent.join(format!("{stem}-{index}.{ext}"))
        };
        if !candidate.exists() {
            return candidate;
        }
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_incremented_name_on_collision() {
        let dir = std::env::temp_dir().join("stream-dl-unique-path");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let base = dir.join("clip.mp4");
        std::fs::write(&base, b"1").unwrap();

        let resolved = unique_path(base);
        assert_eq!(resolved, dir.join("clip-1.mp4"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
