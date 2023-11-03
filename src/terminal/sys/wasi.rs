//! UNIX related logic for terminal manipulation.
use std::io::Error;

use parking_lot::Mutex;

use crate::error::Result;

use wasi_ext_lib::termios;

const STDIN: wasi::Fd = 0x0;

static TERMINAL_MODE_PRIOR_RAW_MODE: Mutex<Option<termios::termios>> = parking_lot::const_mutex(None);

pub(crate) fn is_raw_mode_enabled() -> bool {
    TERMINAL_MODE_PRIOR_RAW_MODE.lock().is_some()
}

pub(crate) fn size() -> Result<(u16, u16)> {
    match wasi_ext_lib::tcgetwinsize(STDIN as wasi::Fd) {
        Ok(winsize) => Ok((winsize.ws_col, winsize.ws_row)),
        Err(e) => Err(Error::from_raw_os_error(e))
    }
}

pub(crate) fn enable_raw_mode() -> Result<()> {
    let mut original_mode = TERMINAL_MODE_PRIOR_RAW_MODE.lock();

    if original_mode.is_some() {
        return Ok(());
    }

    let original_termios = match wasi_ext_lib::tcgetattr(STDIN as wasi::Fd) {
        Ok(term) => term,
        Err(e) => return Err(Error::from_raw_os_error(e)),
    };

    let mut raw_termios = original_termios.clone();
    wasi_ext_lib::cfmakeraw(&mut raw_termios);

    if let Err(e) = wasi_ext_lib::tcsetattr(
        STDIN as wasi::Fd,
        wasi_ext_lib::TcsetattrAction::TCSANOW,
        &raw_termios
    ) {
        return Err(Error::from_raw_os_error(e));
    }

    *original_mode = Some(original_termios);

    Ok(())
}

pub(crate) fn disable_raw_mode() -> Result<()> {
    let mut original_mode = TERMINAL_MODE_PRIOR_RAW_MODE.lock();

    if let Some(original_termios) = original_mode.as_ref() {
        if let Err(e) = wasi_ext_lib::tcsetattr(
            STDIN as wasi::Fd,
            wasi_ext_lib::TcsetattrAction::TCSANOW,
            &original_termios
        ) {
            return Err(Error::from_raw_os_error(e));
        }
        *original_mode = None;
    }

    Ok(())
}
