#[cfg(all(unix, feature = "event-stream"))]
pub(crate) use unix::waker::Waker;
#[cfg(all(windows, feature = "event-stream"))]
pub(crate) use windows::waker::Waker;

#[cfg(unix)]
pub(crate) mod unix;
#[cfg(target_os = "wasi")]
pub(crate) mod wasi;
#[cfg(windows)]
pub(crate) mod windows;
