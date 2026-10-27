//! Platform-independent core of DesktopPet.
//!
//! Everything in here is plain Rust with no windowing or OS calls, so it can
//! be unit-tested on any machine. The Tauri app in `src-tauri` drives these
//! types from its engine loop.

pub mod behavior;
pub mod mood;
pub mod pack;
pub mod reminder;
pub mod settings;
pub mod store;
