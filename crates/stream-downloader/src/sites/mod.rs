//! Site-specific extractors. Register new families in [`registry::SITES`].

pub mod common;
pub mod ok;
pub mod okxxx;
pub mod patterns;
pub mod plugin;
pub mod pornhub;
pub mod registry;
pub mod rutube;
pub mod vk;
pub mod xnxx;
pub mod yandex;
pub mod youtube;

pub use registry::{extract_for_host, refresh_stream};
