use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    #[error("invalid output path: {0}")]
    InvalidOutput(String),

    #[error("no streams found on page")]
    NoStreamsFound,

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("merge error: {0}")]
    Merge(String),
}

pub type Result<T> = std::result::Result<T, Error>;
