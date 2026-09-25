//! Preallocated FIFO: a partial socket write resumes at its exact byte offset.
use std::io::{self, Write};

struct Frame {
    bytes: Box<[u8]>,
    length: usize,
    written: usize,
    deadline: u64,
}
pub(super) struct CompletionQueue {
    frames: Box<[Frame]>,
    head: usize,
    count: usize,
}
impl CompletionQueue {
    pub fn new(capacity: usize, max_bytes: usize) -> Self {
        assert!(capacity > 0 && max_bytes > 0);
        Self {
            frames: (0..capacity)
                .map(|_| Frame {
                    bytes: vec![0; max_bytes].into_boxed_slice(),
                    length: 0,
                    written: 0,
                    deadline: 0,
                })
                .collect(),
            head: 0,
            count: 0,
        }
    }
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    pub fn deadline(&self) -> Option<u64> {
        (!self.is_empty()).then(|| self.frames[self.head].deadline)
    }
    pub fn push(&mut self, bytes: &[u8], now: u64) -> io::Result<()> {
        if self.count == self.frames.len() {
            return Err(io::Error::other("USB completion quota exhausted"));
        }
        let next = (self.head + self.count) % self.frames.len();
        let frame = &mut self.frames[next];
        if bytes.is_empty() || bytes.len() > frame.bytes.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid USB completion size",
            ));
        }
        let deadline = now
            .checked_add(1_000_000)
            .ok_or_else(|| io::Error::other("USB completion clock exhausted"))?;
        frame.bytes[..bytes.len()].copy_from_slice(bytes);
        frame.length = bytes.len();
        frame.written = 0;
        frame.deadline = deadline;
        self.count += 1;
        Ok(())
    }
    pub fn flush(&mut self, writer: &mut impl Write, now: u64) -> io::Result<()> {
        // A hostile writer cannot spin indefinitely on interruptions or tiny writes.
        for _ in 0..64 {
            if self.is_empty() {
                return Ok(());
            }
            let frame = &mut self.frames[self.head];
            if now >= frame.deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "USB completion delivery deadline",
                ));
            }
            match writer.write(&frame.bytes[frame.written..frame.length]) {
                Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
                Ok(n) => frame.written += n,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(()),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => return Ok(()),
                Err(e) => return Err(e),
            }
            if frame.written == frame.length {
                self.head = (self.head + 1) % self.frames.len();
                self.count -= 1;
            }
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    struct Partial {
        bytes: Vec<u8>,
        block: bool,
    }
    impl Write for Partial {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.block = !self.block;
            if !self.block {
                return Err(io::ErrorKind::WouldBlock.into());
            }
            let n = bytes.len().min(2);
            self.bytes.extend_from_slice(&bytes[..n]);
            Ok(n)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    #[test]
    fn partial_writes_preserve_exact_fifo_without_replay() {
        let mut q = CompletionQueue::new(2, 8);
        q.push(b"abcdef", 0).unwrap();
        q.push(b"123", 1).unwrap();
        let mut writer = Partial {
            bytes: vec![],
            block: false,
        };
        for _ in 0..10 {
            q.flush(&mut writer, 2).unwrap();
        }
        assert!(q.is_empty());
        assert_eq!(writer.bytes, b"abcdef123");
        q.push(b"next", 3).unwrap();
        for _ in 0..10 {
            q.flush(&mut writer, 4).unwrap();
        }
        assert_eq!(writer.bytes, b"abcdef123next");
    }
    #[test]
    fn quota_and_absolute_deadline_do_not_reset_on_progress() {
        let mut q = CompletionQueue::new(1, 8);
        assert!(q.push(&[], 0).is_err());
        assert!(q.push(b"123456789", 0).is_err());
        q.push(b"abcdef", 5).unwrap();
        assert!(q.push(b"x", 6).is_err());
        let mut writer = Partial {
            bytes: vec![],
            block: false,
        };
        q.flush(&mut writer, 999_999).unwrap();
        assert_eq!(writer.bytes, b"ab");
        assert_eq!(
            q.flush(&mut writer, 1_000_005).unwrap_err().kind(),
            io::ErrorKind::TimedOut
        );
        assert_eq!(writer.bytes, b"ab");
    }
}
