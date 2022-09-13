//! This module provides platform related functions.
#[cfg(target_os = "wasi")]
pub(crate) use self::wasi::{disable_raw_mode, enable_raw_mode, is_raw_mode_enabled, size};
#[cfg(unix)]
pub(crate) use self::unix::{disable_raw_mode, enable_raw_mode, is_raw_mode_enabled, size};
#[cfg(windows)]
pub(crate) use self::windows::{
    clear, disable_raw_mode, enable_raw_mode, is_raw_mode_enabled, scroll_down, scroll_up,
    set_size, set_window_title, size,
};

#[cfg(target_os = "wasi")]
mod wasi;

#[cfg(windows)]
mod windows;

#[cfg(unix)]
mod unix;
