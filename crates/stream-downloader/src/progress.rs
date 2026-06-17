/// Bytes transferred for one file (or one part of a muxed download).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DownloadProgress<'a> {
    pub label: &'a str,
    pub downloaded: u64,
    pub total: Option<u64>,
}

pub fn format_bytes(n: u64) -> String {
    const K: f64 = 1024.0;
    let b = n as f64;
    if b < K {
        return format!("{n} B");
    }
    if b < K * K {
        return format!("{:.1} KB", b / K);
    }
    if b < K * K * K {
        return format!("{:.1} MB", b / (K * K));
    }
    format!("{:.1} GB", b / (K * K * K))
}

pub fn format_line(p: DownloadProgress<'_>) -> String {
    const W: usize = 24;
    match p.total.filter(|&t| t > 0) {
        Some(total) => {
            let pct = (p.downloaded as f64 / total as f64 * 100.0).min(100.0);
            let filled = ((pct / 100.0) * W as f64).round() as usize;
            let filled = filled.min(W);
            let bar = format!(
                "[{}{}]",
                "█".repeat(filled),
                "░".repeat(W.saturating_sub(filled))
            );
            format!(
                "{}  {:5.1}% {} {}/{}",
                p.label,
                pct,
                bar,
                format_bytes(p.downloaded),
                format_bytes(total)
            )
        }
        None => format!("{}  {} downloaded", p.label, format_bytes(p.downloaded)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_known_total() {
        let line = format_line(DownloadProgress {
            label: "clip.mp4",
            downloaded: 5_000_000,
            total: Some(10_000_000),
        });
        assert!(line.contains("50.0%"));
        assert!(line.contains("4.8 MB/9.5 MB"));
        assert!(line.contains('█'));
    }

    #[test]
    fn formats_unknown_total() {
        let line = format_line(DownloadProgress {
            label: "clip.mp4",
            downloaded: 1024,
            total: None,
        });
        assert!(line.contains("1.0 KB downloaded"));
    }
}
