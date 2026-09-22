#![forbid(unsafe_code)]
#![allow(clippy::missing_errors_doc)]
//! Installed-worker implementation. This crate is not the application API.
#[cfg(target_os = "linux")]
mod session;
#[cfg(target_os = "linux")]
pub use session::{Channels, Setup, run};
