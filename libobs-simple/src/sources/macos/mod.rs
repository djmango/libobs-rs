//! macOS-specific OBS sources
//!
//! Provides safe Rust bindings for macOS capture sources.
//! These wrap OBS's existing mac-capture plugin which handles
//! the actual capture implementation using ScreenCaptureKit.
//!
//! Available sources:
//! - [`ScreenCaptureSourceBuilder`] - Entire screen/monitor capture

pub mod sources;

pub use sources::*;
