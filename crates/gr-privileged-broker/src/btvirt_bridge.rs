//! Fixed stdio contract for the administrator-installed btvirt bridge.

use crate::{BrokerError, HostSession};
use std::{
    fs,
    io::{Read, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    thread,
    time::Duration,
};

pub const BTVIRT_EXECUTABLE: &str = "/usr/libexec/virtualgamepad/virtualgamepad-btvirt";
const VERSION: u8 = 1;
const MAX_PAYLOAD: usize = 128;

/// Encode a bridge message as `u16-le length`, version, tag, body.
pub fn write_frame(writer: &mut impl Write, tag: u8, body: &[u8]) -> Result<(), BrokerError> {
    if body.len() > MAX_PAYLOAD {
        return Err(BrokerError::Host {
            reason: "btvirt bridge payload exceeds bound".into(),
        });
    }
    let length = u16::try_from(body.len() + 2).map_err(|_| BrokerError::Host {
        reason: "btvirt bridge payload exceeds bound".into(),
    })?;
    writer
        .write_all(&length.to_le_bytes())
        .and_then(|()| writer.write_all(&[VERSION, tag]))
        .and_then(|()| writer.write_all(body))
        .map_err(host_io)
}

pub fn read_frame(reader: &mut impl Read) -> Result<(u8, Vec<u8>), BrokerError> {
    let mut length = [0; 2];
    reader.read_exact(&mut length).map_err(host_io)?;
    let length = usize::from(u16::from_le_bytes(length));
    if !(2..=MAX_PAYLOAD + 2).contains(&length) {
        return Err(BrokerError::Host {
            reason: "invalid btvirt bridge frame length".into(),
        });
    }
    let mut frame = vec![0; length];
    reader.read_exact(&mut frame).map_err(host_io)?;
    if frame[0] != VERSION {
        return Err(BrokerError::Host {
            reason: "incompatible btvirt bridge protocol".into(),
        });
    }
    Ok((frame[1], frame[2..].to_vec()))
}

pub struct BtvirtSession {
    child: Child,
    input: ChildStdin,
    output: ChildStdout,
    closed: bool,
}
impl BtvirtSession {
    pub fn open() -> Result<Self, BrokerError> {
        verify_executable(BTVIRT_EXECUTABLE)?;
        let mut child = Command::new(BTVIRT_EXECUTABLE)
            .args(["--broker-stdio", "--protocol=1", "--controller=dualsense"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(host_io)?;
        let input = child.stdin.take().ok_or(BrokerError::Host {
            reason: "btvirt bridge has no stdin".into(),
        })?;
        let mut output = child.stdout.take().ok_or(BrokerError::Host {
            reason: "btvirt bridge has no stdout".into(),
        })?;
        let (tag, body) = read_frame(&mut output)?;
        if tag != 0x80 || body.as_slice() != [1, 1] {
            let _ = child.kill();
            return Err(BrokerError::Host {
                reason: "btvirt bridge lacks Classic HIDP DualSense capability".into(),
            });
        }
        Ok(Self {
            child,
            input,
            output,
            closed: false,
        })
    }

    fn ensure_alive(&mut self) -> Result<(), BrokerError> {
        if let Some(status) = self.child.try_wait().map_err(host_io)? {
            self.closed = true;
            return Err(BrokerError::Host {
                reason: format!("btvirt bridge exited with {status}"),
            });
        }
        Ok(())
    }
}
fn verify_executable(path: &str) -> Result<(), BrokerError> {
    let metadata = fs::metadata(path).map_err(host_io)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if !metadata.is_file()
            || metadata.uid() != 0
            || metadata.mode() & 0o022 != 0
            || metadata.mode() & 0o111 == 0
        {
            return Err(BrokerError::Host {
                reason: "btvirt bridge executable has unsafe ownership or mode".into(),
            });
        }
    }
    Ok(())
}
impl HostSession for BtvirtSession {
    fn send_input(&mut self, report: &[u8]) -> Result<(), BrokerError> {
        self.ensure_alive()?;
        write_frame(&mut self.input, 2, report)
    }
    fn poll_reverse(&mut self) -> Result<Option<Vec<u8>>, BrokerError> {
        self.ensure_alive()?;
        write_frame(&mut self.input, 3, &[])?;
        let (tag, body) = read_frame(&mut self.output)?;
        match tag {
            0x80 => Ok((!body.is_empty()).then_some(body)),
            0x81 => Err(BrokerError::Host {
                reason: "btvirt bridge rejected reverse poll".into(),
            }),
            _ => Err(BrokerError::Host {
                reason: "invalid btvirt bridge reverse response".into(),
            }),
        }
    }
    fn diagnostics(&self) -> Vec<u8> {
        b"btvirt-bridge".to_vec()
    }
    fn close(&mut self) -> Result<(), BrokerError> {
        if !self.closed {
            let _ = write_frame(&mut self.input, 4, &[]);
            for _ in 0..10 {
                if self.child.try_wait().map_err(host_io)?.is_some() {
                    self.closed = true;
                    return Ok(());
                }
                thread::sleep(Duration::from_millis(10));
            }
            self.child.kill().map_err(host_io)?;
            let _ = self.child.wait().map_err(host_io)?;
            self.closed = true;
        }
        Ok(())
    }
}
impl Drop for BtvirtSession {
    fn drop(&mut self) {
        let _ = self.close();
    }
}
#[allow(clippy::needless_pass_by_value)]
fn host_io(error: std::io::Error) -> BrokerError {
    BrokerError::Host {
        reason: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frames_are_versioned_and_bounded() {
        let mut bytes = Vec::new();
        write_frame(&mut bytes, 2, &[1; 78]).unwrap();
        assert_eq!(read_frame(&mut bytes.as_slice()).unwrap(), (2, vec![1; 78]));
        assert!(write_frame(&mut Vec::new(), 2, &[0; 129]).is_err());
    }
    #[test]
    fn missing_bridge_is_rejected_before_process_launch() {
        assert!(verify_executable("/definitely-not-a-virtualgamepad-bridge").is_err());
    }
}
