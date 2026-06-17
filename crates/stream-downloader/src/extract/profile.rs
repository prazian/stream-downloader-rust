use url::Url;

/// How to recognise a site profile from the page host.
#[derive(Debug, Clone, Copy)]
pub enum HostMatch {
    /// `youtube.com` matches `www.youtube.com`.
    Suffix(&'static [&'static str]),
}

impl HostMatch {
    pub fn matches(&self, host: &str) -> bool {
        match self {
            HostMatch::Suffix(suffixes) => suffixes
                .iter()
                .any(|suffix| host == *suffix || host.ends_with(&format!(".{suffix}"))),
        }
    }
}

/// Site-specific discovery rule. Generic HTML rules always run first.
#[derive(Debug, Clone, Copy)]
pub enum ProfileRule {
    /// CSS selector whose attribute holds a direct file URL.
    Selector {
        selector: &'static str,
        attr: &'static str,
        kind: crate::model::MediaKind,
        label_attr: Option<&'static str>,
    },
    /// JavaScript variable embedded in HTML whose JSON lists downloadable files.
    EmbeddedJson(JsonMapRule),
}

/// Maps fields inside JSON format objects to a file URL and metadata.
#[derive(Debug, Clone, Copy)]
pub struct JsonMapRule {
    pub marker: &'static str,
    /// Dot paths to arrays of format objects, e.g. `streamingData.formats`.
    pub item_paths: &'static [&'static str],
    pub url_key: &'static str,
    pub mime_key: Option<&'static str>,
    pub label_key: Option<&'static str>,
    pub alt_label_key: Option<&'static str>,
}

/// Declarative extractor config for one site or family of hosts.
#[derive(Debug, Clone, Copy)]
pub struct SiteProfile {
    pub id: &'static str,
    pub hosts: HostMatch,
    pub rules: &'static [ProfileRule],
}

#[derive(Debug, Clone, Copy)]
pub struct ProfileRegistry {
    profiles: &'static [SiteProfile],
}

impl Default for ProfileRegistry {
    fn default() -> Self {
        Self {
            profiles: crate::extract::profiles::ALL,
        }
    }
}

impl ProfileRegistry {
    pub const fn new(profiles: &'static [SiteProfile]) -> Self {
        Self { profiles }
    }

    pub fn matching<'a>(&'a self, url: &Url) -> impl Iterator<Item = &'a SiteProfile> {
        let host = url.host_str().unwrap_or_default();
        self.profiles
            .iter()
            .filter(move |profile| profile.hosts.matches(host))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suffix_matches_subdomains() {
        let hosts = HostMatch::Suffix(&["youtube.com", "youtu.be"]);
        assert!(hosts.matches("www.youtube.com"));
        assert!(hosts.matches("youtu.be"));
        assert!(!hosts.matches("notyoutube.com"));
    }
}
