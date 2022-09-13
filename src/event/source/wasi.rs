use std::{collections::VecDeque, io::{self, Read}, time::Duration};

use crate::Result;

use super::super::{
    source::EventSource,
    sys::wasi::{
        parse::parse_event,
    },
    InternalEvent,
};

pub(crate) struct WasiInternalEventSource {
    parser: Parser,
    input: io::Stdin,

}

impl WasiInternalEventSource {
    pub fn new() -> Result<Self> {
        // Read only stdin for now
        Ok(
            WasiInternalEventSource {
                parser: Parser::default(),
                input: io::stdin(),
            }
        )
    }

}

impl EventSource for WasiInternalEventSource {
    fn try_read(&mut self, timeout: Option<Duration>) -> Result<Option<InternalEvent>> {
        if let Some(event) = self.parser.next() {
            return Ok(Some(event));
        }

        if let Some(_) = timeout {
            unimplemented!();
        }

        let mut one: [u8; 1] = [0u8; 1];

        loop {
            self.input.read_exact(&mut one).expect("Cannot read stdin!");
            self.parser.advance(&one, true);

            if let Some(event) = self.parser.next() {
                return Ok(Some(event));
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
