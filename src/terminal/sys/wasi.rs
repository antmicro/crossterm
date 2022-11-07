//! UNIX related logic for terminal manipulation.
use std::io::{Error, ErrorKind};

use crate::error::Result;

use sscanf::scanf;

pub(crate) fn is_raw_mode_enabled() -> bool {
    // hterm always has enabled raw mode
    true
}

pub(crate) fn size() -> Result<(u16, u16)> {
    let hterm_screen = match wasi_ext_lib::hterm("screenSize", None) {
        Ok(s) => s,
        Err(e) => return Err(Error::new(ErrorKind::Other, format!("Could not get screen size (os error {})", e)))
    };
    let value = hterm_screen.unwrap();
    match scanf!(value, "[hterm.Size: {}, {}]", u16, u16) {
        Ok(size) => Ok(size),
        Err(e) => {
            println!("{:?}", e);
            Err(
                Error::new(
                    ErrorKind::Unsupported,
                    "Cannot obtain terminal window size with hterm custom syscall"
                )
            )
        }
    }
}

pub(crate) fn enable_raw_mode() -> Result<()> {
    Ok(())
}

pub(crate) fn disable_raw_mode() -> Result<()> {
    Ok(())
}