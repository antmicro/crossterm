use std::{
    collections::VecDeque,
    fs::File,
    io::{self, Read},
    mem,
    time::Duration, os::wasi::prelude::AsRawFd
};
use std::os::wasi::io::{FromRawFd, RawFd};

use wasi;

use crate::Result;

use super::super::{
    source::EventSource,
    sys::wasi::{
        parse::parse_event,
    },
    Event, InternalEvent,
};

const CLOCK_TOKEN: u64 = 1;
const TTY_TOKEN: u64 = 2;
const RESIZE_TOKEN: u64 = 3;

const STDIN: RawFd = 0x0;

const TTY_BUFFER_SIZE: usize = 1_204;

pub(crate) struct WasiInternalEventSource {
    events: [wasi::Event; 3],
    parser: Parser,
    tty_buffer: Vec<u8>,
    tty_input: File,
    event_src: File,

}

impl WasiInternalEventSource {
    pub fn new() -> Result<Self> {
        WasiInternalEventSource::from_file_descriptor(STDIN)
    }

    pub(crate) fn from_file_descriptor(input_fd: RawFd) -> Result<Self> {
        // Read only stdin for now
        let fd_stats = unsafe {
            match wasi::fd_fdstat_get(input_fd as u32) {
                Ok(s) => s,
                Err(e) => {
                    return Err(io::Error::from_raw_os_error(e.raw() as i32))
                }
            }
        };

        // In the wash stdin is char-device with read right
        // Crossterm crate won't panic even if we return Err here
        if fd_stats.fs_filetype != wasi::FILETYPE_CHARACTER_DEVICE ||
            (fd_stats.fs_rights_base & wasi::RIGHTS_FD_READ) == 0 {
            panic!("Polling from fd={} not possible!", input_fd);
        }

        // Obtain hterm event source
        let event_source_fd = {
            match wasi_ext_lib::event_source_fd(
                wasi_ext_lib::WASI_EVENT_WINCH
            ) {
                Ok(fd) => fd,
                Err(e) => {
                    return Err(io::Error::from_raw_os_error(e))
                },
            }
        };

        Ok(
            WasiInternalEventSource {
                events: unsafe { mem::zeroed() },
                parser: Parser::default(),
                tty_buffer: vec![0u8; TTY_BUFFER_SIZE],
                tty_input: unsafe { File::from_raw_fd(input_fd) },
                event_src: unsafe { File::from_raw_fd(event_source_fd) },
            }
        )
    }

}

impl EventSource for WasiInternalEventSource {
    fn try_read(&mut self, timeout: Option<Duration>) -> Result<Option<InternalEvent>> {
        if let Some(event) = self.parser.next() {
            return Ok(Some(event));
        }

        let mut subs = vec![
            wasi::Subscription {
                userdata: TTY_TOKEN,
                u: wasi::SubscriptionU {
                    tag: wasi::EVENTTYPE_FD_READ.raw(),
                    u: wasi::SubscriptionUU {
                        fd_read: wasi::SubscriptionFdReadwrite {
                            file_descriptor: self.tty_input.as_raw_fd() as u32
                        }
                    }
                }
            },
            wasi::Subscription {
                userdata: RESIZE_TOKEN,
                u: wasi::SubscriptionU {
                    tag: wasi::EVENTTYPE_FD_READ.raw(),
                    u: wasi::SubscriptionUU {
                        fd_read: wasi::SubscriptionFdReadwrite {
                            file_descriptor: self.event_src.as_raw_fd() as u32
                        }
                    }
                }
            },
        ];

        if let Some(timeout) = timeout {
            subs.push(wasi::Subscription {
                userdata: CLOCK_TOKEN,
                u: wasi::SubscriptionU {
                    tag: wasi::EVENTTYPE_CLOCK.raw(),
                    u: wasi::SubscriptionUU {
                        clock: wasi::SubscriptionClock {
                            id: wasi::CLOCKID_MONOTONIC,
                            timeout: timeout.as_nanos() as u64,
                            precision: 0,
                            flags: 0
                        }
                    }
                }
            });
        }

        loop {
            // subscribe and wait
            let result = unsafe {
                wasi::poll_oneoff(
                    subs.as_ptr(),
                    self.events.as_mut_ptr(),
                    subs.len()
                )
            };

            let events_count = match result {
                Ok(n) => n,
                Err(e) => {
                    return Err(io::Error::from_raw_os_error(e.raw() as i32));
                }
            };

            if events_count == 0 {
                return Ok(None)
            }

            // iterate over occured events
            for event in self.events[0..events_count].iter() {
                let errno = event.error.raw();
                if errno > 0 {
                    return Err(io::Error::from_raw_os_error(errno as i32))
                }
            }

            for event in self.events[0..events_count].iter() {
                match (event.userdata, event.type_) {
                    (CLOCK_TOKEN, wasi::EVENTTYPE_CLOCK) => {
                        return Ok(None)
                    },
                    (TTY_TOKEN, wasi::EVENTTYPE_FD_READ) => {
                        let to_read = event.fd_readwrite.nbytes as usize;
                        if to_read > self.tty_buffer.len() {
                            self.tty_buffer.resize(to_read, 0);
                        }
                        let read_bytes = match self.tty_input.read(&mut self.tty_buffer[0..to_read]) {
                            Ok(n) => n,
                            Err(e) => {
                                return Err(e);
                            }
                        };

                        let more = read_bytes == self.tty_buffer.len();
                        self.parser.advance(
                            &mut self.tty_buffer[0..read_bytes],
                            more
                        );

                        if let Some(event) = self.parser.next() {
                            return Ok(Some(event));
                        }
                    },
                    (RESIZE_TOKEN, wasi::EVENTTYPE_FD_READ) => {
                        let to_read = event.fd_readwrite.nbytes as usize;
                        let mut read_buff: [u8; wasi_ext_lib::WASI_EVENTS_MASK_SIZE] = [
                            0u8; wasi_ext_lib::WASI_EVENTS_MASK_SIZE
                        ];

                        if let Err(e) = self.event_src.read(&mut read_buff[0..to_read]) {
                            return Err(e);
                        };

                        let events = read_buff[0] as wasi_ext_lib::WasiEvents;

                        if events & wasi_ext_lib::WASI_EVENT_WINCH != 0 {
                            let new_size = crate::terminal::size()?;
                            return Ok(Some(InternalEvent::Event(Event::Resize(
                                new_size.0, new_size.1,
                            ))));
                        }
                    },
                    _ => unreachable!(),
                }
            }
        }
    }
}

//
// Following `Parser` structure exists for two reasons:
//
//  * mimic anes Parser interface
//  * move the advancing, parsing, ... stuff out of the `try_read` method
//
#[derive(Debug)]
struct Parser {
    buffer: Vec<u8>,
    internal_events: VecDeque<InternalEvent>,
}

impl Default for Parser {
    fn default() -> Self {
        Parser {
            buffer: Vec::with_capacity(256),
            internal_events: VecDeque::with_capacity(128),
        }
    }
}

impl Parser {
    fn advance(&mut self, buffer: &[u8], more: bool) {
        for (idx, byte) in buffer.iter().enumerate() {
            let more = idx + 1 < buffer.len() || more;

            self.buffer.push(*byte);

            match parse_event(&self.buffer, more) {
                Ok(Some(ie)) => {
                    self.internal_events.push_back(ie);
                    self.buffer.clear();
                }
                Ok(None) => {
                    // Event can't be parsed, because we don't have enough bytes for
                    // the current sequence. Keep the buffer and process next bytes.
                }
                Err(_) => {
                    // Event can't be parsed (not enough parameters, parameter is not a number, ...).
                    // Clear the buffer and continue with another sequence.
                    self.buffer.clear();
                }
            }
        }
    }
}

impl Iterator for Parser {
    type Item = InternalEvent;

    fn next(&mut self) -> Option<Self::Item> {
        self.internal_events.pop_front()
    }
}
