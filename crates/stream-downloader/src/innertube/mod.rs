//! YouTube innertube player API (shared by site plugins).

mod client;
mod config;
mod formats;
mod page;

pub use client::player;
pub use config::ClientConfig;
pub use formats::select_for_kinds;
pub use page::PageKeys;
