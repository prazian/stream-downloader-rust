use crate::client;
use crate::error::Result;
use crate::merge;
use crate::model::{MediaKind, MuxPart, Stream};
use crate::naming::OutputName;
use crate::progress::DownloadProgress;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::task::JoinSet;

const MIN_PARALLEL_BYTES: u64 = 2 * 1024 * 1024;
const MIN_CHUNK_BYTES: u64 = 8 * 1024 * 1024;
const MAX_PARALLEL_CHUNKS: usize = 16;
const PROGRESS_INTERVAL: Duration = Duration::from_millis(200);

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
        Self {
            client,
        }
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
                .download_muxed(
                    stream,
                    audio,
                    &path,
                    options,
                    &name.file_name,
                )
                .await;
        }

        if stream.hls {
            return self
                .download_hls(stream, &path, options)
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
        label: &str,
    ) -> Result<PathBuf> {
        let stem = output
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("out");
        let dir = output
            .parent()
            .unwrap_or_else(|| Path::new("."));
        let video_part = dir.join(format!(".{stem}-video.part"));
        let audio_part = dir.join(format!(".{stem}-audio.part"));

        let video_label = format!("{label} (video)");
        let audio_label = format!("{label} (audio)");
        self.download_to_path(
            video,
            &video_part,
            options,
            &video_label,
        )
        .await?;
        self.download_to_path(
            &Stream {
                url: audio.url.clone(),
                kind: MediaKind::Audio,
                label: None,
                download_user_agent: audio.download_user_agent,
                mux_audio: None,
                hls: false,
                download_referer: None,
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

    async fn download_hls(
        &self,
        stream: &Stream,
        path: &Path,
        options: &DownloadOptions<'_>,
    ) -> Result<PathBuf> {
        if let Some(cb) = options.on_progress {
            cb(crate::progress::DownloadProgress {
                label: "Downloading HLS…",
                downloaded: 0,
                total: None,
            });
        }
        let url = stream.url.to_string();
        let referer = stream
            .download_referer
            .or(options.referer)
            .map(str::to_owned);
        let output = path.to_path_buf();
        let out = output.clone();
        tokio::task::spawn_blocking(move || {
            merge::download_hls(&url, &out, referer.as_deref())
        })
        .await
        .map_err(|e| crate::error::Error::Merge(e.to_string()))??;
        Ok(output)
    }

    async fn download_to_path(
        &self,
        stream: &Stream,
        path: &Path,
        options: &DownloadOptions<'_>,
        label: &str,
    ) -> Result<()> {
        let (total, ranges) = self.probe(stream, options).await?;
        if ranges && total.is_some_and(|t| t >= MIN_PARALLEL_BYTES) {
            match self
                .download_parallel(
                    stream,
                    path,
                    options,
                    label,
                    total.unwrap(),
                )
                .await
            {
                | Ok(()) => return Ok(()),
                | Err(e) if retriable_http(&e) => {
                    let _ = remove_parallel_parts(path);
                },
                | Err(e) => return Err(e),
            }
        }

        self.download_sequential(stream, path, options, label, total)
            .await
    }

    async fn download_sequential(
        &self,
        stream: &Stream,
        path: &Path,
        options: &DownloadOptions<'_>,
        label: &str,
        total_hint: Option<u64>,
    ) -> Result<()> {
        let mut response = send_with_retry(
            self.build_request(stream, options),
            None,
        )
        .await?;
        let total = total_hint.or_else(|| response.content_length());
        let mut downloaded = 0u64;
        let mut last_pct = None::<u32>;
        report(
            options,
            label,
            downloaded,
            total,
            &mut last_pct,
        );

        let mut file = tokio::fs::File::create(path).await?;
        while let Some(chunk) = response.chunk().await? {
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;
            report(
                options,
                label,
                downloaded,
                total,
                &mut last_pct,
            );
        }
        file.flush().await?;
        report(
            options,
            label,
            downloaded,
            total,
            &mut last_pct,
        );
        Ok(())
    }

    async fn probe(
        &self,
        stream: &Stream,
        options: &DownloadOptions<'_>,
    ) -> Result<(Option<u64>, bool)> {
        let response = send_with_retry(
            self.build_request(stream, options)
                .header(reqwest::header::RANGE, "bytes=0-0"),
            None,
        )
        .await?;
        let total = content_range_total(response.headers())
            .or_else(|| response.content_length());
        let ranges = response
            .headers()
            .get(reqwest::header::ACCEPT_RANGES)
            .or_else(|| {
                response
                    .headers()
                    .get(reqwest::header::CONTENT_RANGE)
            })
            .is_some();
        Ok((total, ranges))
    }

    fn build_request(
        &self,
        stream: &Stream,
        options: &DownloadOptions<'_>,
    ) -> reqwest::RequestBuilder {
        let mut request = self.client.get(stream.url.clone());
        let referer = stream
            .download_referer
            .or(options.referer);
        if let Some(referer) = referer {
            request = request.header(reqwest::header::REFERER, referer);
        }
        if let Some(ua) = stream.download_user_agent {
            request = request.header(reqwest::header::USER_AGENT, ua);
        }
        request
    }

    async fn download_parallel(
        &self,
        stream: &Stream,
        path: &Path,
        options: &DownloadOptions<'_>,
        label: &str,
        total: u64,
    ) -> Result<()> {
        let chunks = parallel_chunks(total, &stream.url);
        let chunk_size = total.div_ceil(chunks as u64);
        let parent = path
            .parent()
            .unwrap_or_else(|| Path::new("."));
        let stem = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("download");
        let mut parts = Vec::with_capacity(chunks);
        let mut set = JoinSet::new();
        let downloaded = Arc::new(AtomicU64::new(0));
        let mut last_pct = None::<u32>;
        let mut interval = tokio::time::interval(PROGRESS_INTERVAL);
        interval
            .set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        report(
            options,
            label,
            0,
            Some(total),
            &mut last_pct,
        );

        for i in 0..chunks {
            let start = i as u64 * chunk_size;
            if start >= total {
                break;
            }
            let end = (start + chunk_size).min(total) - 1;
            let part = parent.join(format!(".{stem}.part{i}"));
            parts.push(part.clone());

            let client = self.client.clone();
            let url = stream.url.clone();
            let referer = stream
                .download_referer
                .or(options.referer)
                .map(str::to_owned);
            let ua = stream
                .download_user_agent
                .map(str::to_owned);
            let counter = Arc::clone(&downloaded);
            set.spawn(async move {
                let range = format!("bytes={start}-{end}");
                let mut response = send_with_retry(
                    {
                        let mut req = client
                            .get(url)
                            .header(reqwest::header::RANGE, range.clone());
                        if let Some(referer) = &referer {
                            req = req.header(reqwest::header::REFERER, referer);
                        }
                        if let Some(ua) = &ua {
                            req = req.header(reqwest::header::USER_AGENT, ua);
                        }
                        req
                    },
                    Some(range),
                )
                .await?;
                let mut file = tokio::fs::File::create(&part).await?;
                while let Some(chunk) = response.chunk().await? {
                    file.write_all(&chunk).await?;
                    counter.fetch_add(chunk.len() as u64, Ordering::Relaxed);
                }
                file.flush().await?;
                Ok::<_, crate::error::Error>(())
            });
        }

        loop {
            tokio::select! {
                result = set.join_next() => {
                    match result {
                        None => break,
                        Some(Ok(Ok(()))) => {}
                        Some(Ok(Err(e))) => return Err(e),
                        Some(Err(e)) => {
                            return Err(crate::error::Error::Merge(e.to_string()));
                        }
                    }
                }
                _ = interval.tick(), if options.on_progress.is_some() => {
                    report(
                        options,
                        label,
                        downloaded.load(Ordering::Relaxed),
                        Some(total),
                        &mut last_pct,
                    );
                }
            }
        }

        let mut out = tokio::fs::File::create(path).await?;
        for part in &parts {
            let mut part_file = tokio::fs::File::open(part).await?;
            tokio::io::copy(&mut part_file, &mut out).await?;
            let _ = tokio::fs::remove_file(part).await;
        }
        out.flush().await?;
        report(
            options,
            label,
            total,
            Some(total),
            &mut last_pct,
        );
        Ok(())
    }
}

fn parallel_chunks(
    total: u64,
    url: &url::Url,
) -> usize {
    let mut chunks = match total {
        | n if n < 20 * 1024 * 1024 => 2,
        | n if n < 100 * 1024 * 1024 => 4,
        | n if n < 512 * 1024 * 1024 => 8,
        | _ => 16,
    };
    let by_chunk_size = (total / MIN_CHUNK_BYTES) as usize;
    chunks = chunks
        .min(by_chunk_size)
        .clamp(1, MAX_PARALLEL_CHUNKS);
    if url
        .host_str()
        .is_some_and(|h| h.contains("phncdn.com") || h.contains("xnxx-cdn.com"))
    {
        chunks = chunks.min(4);
    }
    chunks
}

const RETRYABLE_STATUS: [u16; 3] = [429, 502, 503];
const RETRY_ATTEMPTS: u32 = 4;

async fn send_with_retry(
    builder: reqwest::RequestBuilder,
    range: Option<String>,
) -> Result<reqwest::Response> {
    let mut delay = Duration::from_millis(400);
    let mut last = None;
    for attempt in 0..RETRY_ATTEMPTS {
        let mut req = builder.try_clone().ok_or_else(|| {
            crate::error::Error::Merge("failed to clone HTTP request".into())
        })?;
        if let Some(range) = &range {
            req = req.header(reqwest::header::RANGE, range);
        }
        match req.send().await {
            | Ok(response) => {
                let status = response.status();
                if RETRYABLE_STATUS.contains(&status.as_u16())
                    && attempt + 1 < RETRY_ATTEMPTS
                {
                    tokio::time::sleep(delay).await;
                    delay *= 2;
                    continue;
                }
                return response
                    .error_for_status()
                    .map_err(Into::into);
            },
            | Err(e)
                if (e.is_timeout() || e.is_connect())
                    && attempt + 1 < RETRY_ATTEMPTS =>
            {
                last = Some(crate::error::Error::Http(e));
                tokio::time::sleep(delay).await;
                delay *= 2;
            },
            | Err(e) => return Err(e.into()),
        }
    }
    Err(last.unwrap_or_else(|| {
        crate::error::Error::Merge("HTTP request failed after retries".into())
    }))
}

fn retriable_http(err: &crate::error::Error) -> bool {
    matches!(
        err,
        crate::error::Error::Http(e)
            if e.status()
                .is_some_and(|s| RETRYABLE_STATUS.contains(&s.as_u16()))
    )
}

fn remove_parallel_parts(path: &Path) -> std::io::Result<()> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    let Some(stem) = path
        .file_name()
        .and_then(|s| s.to_str())
    else {
        return Ok(());
    };
    for entry in std::fs::read_dir(parent)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(&format!(".{stem}.part")) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
    Ok(())
}

fn content_range_total(headers: &reqwest::header::HeaderMap) -> Option<u64> {
    let value = headers
        .get(reqwest::header::CONTENT_RANGE)?
        .to_str()
        .ok()?;
    let total = value.split('/').nth(1)?;
    total.parse().ok()
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
        | Some(t) => {
            let pct = (downloaded.saturating_mul(100) / t).min(100) as u32;
            if downloaded < t && Some(pct) == *last_pct {
                return;
            }
            *last_pct = Some(pct);
        },
        | None => {
            let bucket = (downloaded / (512 * 1024)) as u32;
            if downloaded > 0 && Some(bucket) == *last_pct {
                return;
            }
            *last_pct = Some(bucket);
        },
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
    let Some(stem) = path
        .file_stem()
        .and_then(|s| s.to_str())
    else {
        return path;
    };
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let parent = path
        .parent()
        .unwrap_or_else(|| Path::new("."));

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
    fn adaptive_parallel_chunks() {
        let ph = url::Url::parse("https://ev.phncdn.com/v.mp4").unwrap();
        let cdn = url::Url::parse("https://cdn.example.com/v.mp4").unwrap();
        assert_eq!(
            parallel_chunks(5 * 1024 * 1024, &cdn),
            1
        );
        assert_eq!(
            parallel_chunks(20 * 1024 * 1024, &cdn),
            2
        );
        assert_eq!(
            parallel_chunks(50 * 1024 * 1024, &cdn),
            4
        );
        assert_eq!(
            parallel_chunks(200 * 1024 * 1024, &cdn),
            8
        );
        assert_eq!(
            parallel_chunks(2 * 1024 * 1024 * 1024, &cdn),
            16
        );
        assert_eq!(
            parallel_chunks(2 * 1024 * 1024 * 1024, &ph),
            4
        );
    }

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
