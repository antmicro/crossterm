//! UNIX related logic for terminal manipulation.

use std::fs;
use std::io::{Error, ErrorKind};

use crate::error::Result;

use serde_json::json;
use sscanf::scanf;

pub(crate) fn is_raw_mode_enabled() -> bool {
    // hterm always has enabled raw mode
    true
}

pub(crate) fn size() -> Result<(u16, u16)> {
    let custom_syscall = json!({
        "command": "hterm",
        "args": &["get", "screenSize"],
    });
    let hterm_screen = fs::read_link(format!("/!{}", custom_syscall))?;
    let value = hterm_screen.display().to_string();
    match scanf!(value, "0\u{1b}[hterm.Size: {}, {}]", u16, u16) {
        Ok(size) => Ok(size),
        Err(_) => Err(
                Error::new(
                    ErrorKind::Unsupported,
                    "Cannot obtain terminal window size with hterm custom syscall"
                )
            ),
    }
}

pub(crate) fn enable_raw_mode() -> Result<()> {
    Ok(())
}

pub(crate) fn disable_raw_mode() -> Result<()> {
    Ok(())
}