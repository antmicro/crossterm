//! UNIX related logic for terminal manipulation.
use std::io::{Error, ErrorKind, stdin};
use std::os::fd::AsRawFd;

use crate::error::Result;

use wasi_ext_lib::{ioctl, IoctlNum};

pub(crate) fn is_raw_mode_enabled() -> bool {
    // hterm always has enabled raw mode
    true
}

pub(crate) fn size() -> Result<(u16, u16)> {
    let mut size = [0i32; 2];
    match ioctl(stdin().as_raw_fd(), IoctlNum::GetScreenSize , Some(&mut size)) {
        Ok(()) => Ok((size[0] as u16, size[1] as u16)),
        Err(e) => Err(Error::new(ErrorKind::Other, format!("Could not get screen size (os error {})", e)))
    }
}

pub(crate) fn enable_raw_mode() -> Result<()> {
    Ok(())
}

pub(crate) fn disable_raw_mode() -> Result<()> {
    Ok(())
}
