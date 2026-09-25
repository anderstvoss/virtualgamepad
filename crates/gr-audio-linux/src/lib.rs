#![forbid(unsafe_code)]
//! Linux audio transport SPI. Controller profiles and routing policy live elsewhere.
#[cfg(all(target_os = "linux", feature = "pipewire"))]
mod pipewire_backend;
#[cfg(all(target_os = "linux", feature = "pipewire"))]
pub use pipewire_backend::*;

#[cfg(all(target_os = "linux", feature = "pipewire"))]
mod timing;
#[cfg(all(target_os = "linux", feature = "pipewire"))]
pub use timing::StreamTiming;
