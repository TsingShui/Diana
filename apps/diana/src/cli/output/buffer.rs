//! Fallibly allocated output buffering with explicit flush and no drop-time I/O.

use std::io::{self, Write};

use super::Failure;
use crate::analysis::input::InputError;

pub(in crate::cli) struct Buffered<'a> {
    sink: &'a mut dyn Write,
    bytes: Vec<u8>,
    written: usize,
    failed: Option<io::ErrorKind>,
}

impl<'a> Buffered<'a> {
    pub(super) fn new(sink: &'a mut dyn Write) -> Result<Self, Failure> {
        let mut bytes = Vec::new();
        // A transfer buffer, not an output limit. Larger writes bypass it.
        bytes
            .try_reserve_exact(1 << 18)
            .map_err(|error| Failure::input(InputError::Allocation(error)))?;
        Ok(Self {
            sink,
            bytes,
            written: 0,
            failed: None,
        })
    }

    pub(super) fn failed(&self) -> bool {
        self.failed.is_some()
    }

    fn available(&self) -> io::Result<()> {
        match self.failed {
            Some(kind) => Err(io::Error::new(kind, "result output previously failed")),
            None => Ok(()),
        }
    }

    fn record<T>(&mut self, result: io::Result<T>) -> io::Result<T> {
        if let Err(error) = &result
            && error.kind() != io::ErrorKind::Interrupted
        {
            self.failed = Some(error.kind());
        }
        result
    }

    fn drain(&mut self) -> io::Result<()> {
        self.available()?;
        while self.written < self.bytes.len() {
            match self.sink.write(&self.bytes[self.written..]) {
                Ok(0) => {
                    return self.record(Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "result output accepted no bytes",
                    )));
                }
                // Write's contract bounds this count by the supplied slice.
                Ok(count) => self.written += count,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return self.record(Err(error)),
            }
        }
        self.bytes.clear();
        self.written = 0;
        Ok(())
    }
}

impl Write for Buffered<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.available()?;
        if bytes.len() > self.bytes.capacity() - self.bytes.len() {
            self.drain()?;
        }
        if bytes.len() >= self.bytes.capacity() {
            let result = match self.sink.write(bytes) {
                Ok(0) => Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "result output accepted no bytes",
                )),
                result => result,
            };
            return self.record(result);
        }
        // The complete fragment fits the reserved capacity; appending cannot grow.
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    #[inline]
    fn write_all(&mut self, bytes: &[u8]) -> io::Result<()> {
        // Keep token-sized appends cheap. A sticky sink failure is still reported
        // before draining or final flush, so buffered acceptance is not delivery.
        if bytes.len() <= self.bytes.capacity() - self.bytes.len() {
            self.bytes.extend_from_slice(bytes);
            return Ok(());
        }
        self.drain()?;
        if bytes.len() >= self.bytes.capacity() {
            let result = self.sink.write_all(bytes);
            return self.record(result);
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.drain()?;
        let result = self.sink.flush();
        self.record(result)
    }
}
