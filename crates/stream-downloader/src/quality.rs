use crate::model::Stream;
use std::cmp::Ordering;

/// Which file(s) to pick when a page offers multiple resolutions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Quality {
    #[default]
    Best,
    All,
    Height(u32),
}

impl Quality {
    pub fn parse(raw: &str) -> crate::error::Result<Self> {
        let s = raw.trim();
        if s.eq_ignore_ascii_case("all") {
            return Ok(Self::All);
        }
        if s.eq_ignore_ascii_case("best") {
            return Ok(Self::Best);
        }
        if s.eq_ignore_ascii_case("4k") {
            return Ok(Self::Height(2160));
        }
        let height = s
            .trim_end_matches(['p', 'P'])
            .parse()
            .map_err(|_| {
                crate::error::Error::InvalidUrl(format!("bad quality `{raw}`"))
            })?;
        Ok(Self::Height(height))
    }

    pub fn pick<'a>(
        &self,
        heights: impl IntoIterator<Item = &'a u32>,
    ) -> Vec<u32> {
        let heights: Vec<u32> = heights.into_iter().copied().collect();
        match self {
            | Self::All => heights,
            | Self::Best => heights
                .into_iter()
                .max()
                .into_iter()
                .collect(),
            | Self::Height(h) => heights
                .into_iter()
                .filter(|x| x == h)
                .collect(),
        }
    }

    pub fn filter_streams(
        &self,
        streams: Vec<Stream>,
    ) -> Vec<Stream> {
        if matches!(self, Self::All) {
            return streams;
        }
        let heights: Vec<u32> = streams
            .iter()
            .map(height_hint)
            .collect();
        let pick = self.pick(heights.iter());
        streams
            .into_iter()
            .filter(|s| pick.contains(&height_hint(s)))
            .collect()
    }
}

pub fn height_hint(stream: &Stream) -> u32 {
    stream
        .label
        .as_deref()
        .and_then(|l| l.trim_end_matches('p').parse().ok())
        .unwrap_or(0)
}

/// Pick by height (highest first), falling back to the next lower resolution.
pub fn pick_streams(
    streams: Vec<Stream>,
    quality: Quality,
) -> crate::error::Result<Vec<Stream>> {
    pick_sorted(streams, quality, |a, b| {
        height_hint(b).cmp(&height_hint(a))
    })
}

/// Like [`pick_streams`], but prefers progressive files over HLS at the same height (VK).
pub fn pick_streams_prefer_progressive(
    streams: Vec<Stream>,
    quality: Quality,
) -> crate::error::Result<Vec<Stream>> {
    pick_sorted(streams, quality, |a, b| {
        match (a.hls, b.hls) {
            | (true, false) => Ordering::Greater,
            | (false, true) => Ordering::Less,
            | _ => height_hint(b).cmp(&height_hint(a)),
        }
    })
}

fn pick_sorted(
    mut streams: Vec<Stream>,
    quality: Quality,
    compare: impl Fn(&Stream, &Stream) -> Ordering,
) -> crate::error::Result<Vec<Stream>> {
    if streams.is_empty() {
        return Err(crate::error::Error::NoStreamsFound);
    }
    streams.sort_by(compare);
    let picked = match quality {
        | Quality::All => streams,
        | Quality::Best => vec![streams.remove(0)],
        | Quality::Height(want) => {
            if let Some(stream) = streams
                .iter()
                .find(|s| height_hint(s) == want)
            {
                vec![stream.clone()]
            } else {
                streams
                    .into_iter()
                    .filter(|s| height_hint(s) <= want)
                    .take(1)
                    .collect()
            }
        },
    };
    if picked.is_empty() {
        Err(crate::error::Error::NoStreamsFound)
    } else {
        Ok(picked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_labels() {
        assert_eq!(
            Quality::parse("1080p").unwrap(),
            Quality::Height(1080)
        );
        assert_eq!(
            Quality::parse("4k").unwrap(),
            Quality::Height(2160)
        );
        assert_eq!(
            Quality::parse("all").unwrap(),
            Quality::All
        );
    }

    #[test]
    fn picks_heights() {
        let hs = [144u32, 360, 720, 1080];
        assert_eq!(
            Quality::Best.pick(hs.iter()),
            vec![1080]
        );
        assert_eq!(
            Quality::Height(720).pick(hs.iter()),
            vec![720]
        );
        assert_eq!(Quality::All.pick(hs.iter()).len(), 4);
    }

    #[test]
    fn picks_streams_with_fallback() {
        let streams = vec![stream(720), stream(1080), stream(480)];
        let picked = pick_streams(streams, Quality::Height(1080)).unwrap();
        assert_eq!(picked.len(), 1);
        assert_eq!(height_hint(&picked[0]), 1080);

        let streams = vec![stream(720), stream(480)];
        let picked = pick_streams(streams, Quality::Height(1080)).unwrap();
        assert_eq!(height_hint(&picked[0]), 720);
    }

    #[test]
    fn prefers_progressive_over_hls() {
        let streams = vec![
            stream_hls(720),
            Stream {
                hls: false,
                ..stream(720)
            },
        ];
        let picked =
            pick_streams_prefer_progressive(streams, Quality::Best).unwrap();
        assert!(!picked[0].hls);
    }

    fn stream(h: u32) -> Stream {
        Stream {
            url: url::Url::parse("https://example.com/v.mp4").unwrap(),
            kind: crate::model::MediaKind::Video,
            label: Some(format!("{h}p")),
            download_user_agent: None,
            mux_audio: None,
            hls: false,
            download_referer: None,
        }
    }

    fn stream_hls(h: u32) -> Stream {
        Stream {
            hls: true,
            ..stream(h)
        }
    }
}
