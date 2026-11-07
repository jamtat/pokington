//! Probe/test-only writer for one partial-output failure followed by normal cleanup writes.
use std::io::{self, Write};

use serde::Serialize;

#[derive(Serialize)]
pub struct FaultReport {
    pub prefix_bytes: usize,
    pub forwarded_before_failure: usize,
    pub injected: bool,
    pub forwarded_after_failure: usize,
    pub cleanup_complete: bool,
}

pub struct FailOnce<W> {
    inner: W,
    prefix_bytes: usize,
    forwarded: usize,
    failed: bool,
    recovery: Vec<u8>,
    recovery_flushed: bool,
}

impl<W: Write> FailOnce<W> {
    pub fn after_bytes(inner: W, prefix_bytes: usize) -> Self {
        Self {
            inner,
            prefix_bytes,
            forwarded: 0,
            failed: false,
            recovery: Vec::new(),
            recovery_flushed: false,
        }
    }

    pub fn inside_first_apc(inner: W, frame: &[u8]) -> io::Result<Self> {
        let apc = frame
            .windows(3)
            .position(|bytes| bytes == b"\x1b_G")
            .ok_or_else(|| io::Error::other("no Kitty APC available for fault injection"))?;
        // End inside the first command header, before its semicolon/payload and terminator.
        let prefix_bytes = apc + 3 + b"q=2,".len();
        if frame.get(apc + 3..prefix_bytes) != Some(b"q=2,") {
            return Err(io::Error::other(
                "unexpected Kitty APC header for fault injection",
            ));
        }
        Ok(Self::after_bytes(inner, prefix_bytes))
    }

    pub fn report(&self) -> FaultReport {
        FaultReport {
            prefix_bytes: self.prefix_bytes,
            forwarded_before_failure: self.forwarded,
            injected: self.failed,
            forwarded_after_failure: self.recovery.len(),
            cleanup_complete: self.recovery_flushed && self.recovery == b"\x18\x1b\\\x1b[0m\r\n",
        }
    }
}

impl<W: Write> Write for FailOnce<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.forwarded == self.prefix_bytes && !self.failed {
            self.failed = true;
            // Ensure the incomplete APC reaches the terminal before returning the failure.
            self.inner.flush()?;
            return Err(io::Error::other(
                "injected failure inside native Kitty output",
            ));
        }
        let len = if self.failed {
            bytes.len()
        } else {
            bytes.len().min(self.prefix_bytes - self.forwarded)
        };
        let count = self.inner.write(&bytes[..len])?;
        if self.failed {
            self.recovery.extend_from_slice(&bytes[..count]);
        } else {
            self.forwarded += count;
        }
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()?;
        if self.failed {
            self.recovery_flushed = true;
        }
        Ok(())
    }
}
