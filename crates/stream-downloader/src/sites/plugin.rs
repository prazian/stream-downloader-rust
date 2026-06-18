//! Site plugin types — one [`SitePlugin`] per host family, registered in [`super::registry`].

use crate::error::Result;
use crate::extract::ExtractOptions;
use crate::model::{FetchedPage, Stream};
use std::future::Future;
use std::pin::Pin;
use url::Url;

pub type ExtractFn = for<'a> fn(
    &'a reqwest::Client,
    &'a FetchedPage,
    &'a ExtractOptions,
) -> Pin<
    Box<dyn Future<Output = Result<Vec<Stream>>> + Send + 'a>,
>;

pub type RefreshFn = for<'a> fn(
    &'a reqwest::Client,
    &'a FetchedPage,
    &'a Stream,
) -> Pin<
    Box<dyn Future<Output = Result<Stream>> + Send + 'a>,
>;

/// One registered site. Add a row to [`super::registry::SITES`] to wire a new module.
#[derive(Clone, Copy)]
pub struct SitePlugin {
    /// Stable identifier (logging, tests).
    #[allow(dead_code)]
    pub id: &'static str,
    pub matches: fn(&Url) -> bool,
    pub extract: ExtractFn,
    pub refresh: Option<RefreshFn>,
}

impl SitePlugin {
    pub const fn new(
        id: &'static str,
        matches: fn(&Url) -> bool,
        extract: ExtractFn,
        refresh: Option<RefreshFn>,
    ) -> Self {
        Self {
            id,
            matches,
            extract,
            refresh,
        }
    }
}
