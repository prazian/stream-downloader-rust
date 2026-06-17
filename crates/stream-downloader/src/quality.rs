use crate::model::Stream;

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
            .map_err(|_| crate::error::Error::InvalidUrl(format!("bad quality `{raw}`")))?;
        Ok(Self::Height(height))
    }

    pub fn pick<'a>(&self, heights: impl IntoIterator<Item = &'a u32>) -> Vec<u32> {
        let heights: Vec<u32> = heights.into_iter().copied().collect();
        match self {
            Self::All => heights,
            Self::Best => heights.into_iter().max().into_iter().collect(),
            Self::Height(h) => heights.into_iter().filter(|x| x == h).collect(),
        }
    }

    pub fn filter_streams(&self, streams: Vec<Stream>) -> Vec<Stream> {
        if matches!(self, Self::All) {
            return streams;
        }
        let heights: Vec<u32> = streams.iter().map(height_hint).collect();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_labels() {
        assert_eq!(Quality::parse("1080p").unwrap(), Quality::Height(1080));
        assert_eq!(Quality::parse("4k").unwrap(), Quality::Height(2160));
        assert_eq!(Quality::parse("all").unwrap(), Quality::All);
    }

    #[test]
    fn picks_heights() {
        let hs = [144u32, 360, 720, 1080];
        assert_eq!(Quality::Best.pick(hs.iter()), vec![1080]);
        assert_eq!(Quality::Height(720).pick(hs.iter()), vec![720]);
        assert_eq!(Quality::All.pick(hs.iter()).len(), 4);
    }
}
