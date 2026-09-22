//! Bounded USB/IP data-phase codec for a local, trusted device worker.
//! This is transport SPI, not a remote USB server or an enabled realization.
//! Buffers are caller-owned; decoding and encoding allocate no memory.
#![forbid(unsafe_code)]
#![allow(clippy::missing_errors_doc)]

pub mod control;
#[cfg(unix)]
pub mod pcm_ipc;
pub mod pending;
pub mod profile;
#[cfg(unix)]
pub mod worker;

pub const HEADER_BYTES: usize = 48;
pub const MAX_TRANSFER_BYTES: usize = 65_536;
pub const MAX_ISO_PACKETS: usize = 1024;
pub const MAX_FRAME_BYTES: usize = HEADER_BYTES + MAX_TRANSFER_BYTES + MAX_ISO_PACKETS * 16;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Incomplete,
    Malformed,
    Limit,
    WrongDevice,
    ReplyLength,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Out,
    In,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    sequence: u32,
    device: u32,
    endpoint: u8,
    direction: Direction,
    transfer_flags: u32,
    transfer_bytes: usize,
    start_frame: u32,
    interval: u32,
    setup: [u8; 8],
    packets: usize,
    unlink: Option<u32>,
}
fn word(bytes: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("checked header bounds"),
    )
}
fn put(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}
impl Header {
    #[must_use]
    pub const fn sequence(self) -> u32 {
        self.sequence
    }
    #[must_use]
    pub const fn device(self) -> u32 {
        self.device
    }
    #[must_use]
    pub const fn endpoint(self) -> u8 {
        self.endpoint
    }
    #[must_use]
    pub const fn direction(self) -> Direction {
        self.direction
    }
    #[must_use]
    pub const fn transfer_bytes(self) -> usize {
        self.transfer_bytes
    }
    #[must_use]
    pub const fn setup(self) -> [u8; 8] {
        self.setup
    }
    #[must_use]
    pub const fn interval(self) -> u32 {
        self.interval
    }
    #[must_use]
    pub const fn transfer_flags(self) -> u32 {
        self.transfer_flags
    }
    pub fn decode(bytes: &[u8], expected_device: u32) -> Result<Self, Error> {
        if bytes.len() < HEADER_BYTES {
            return Err(Error::Incomplete);
        }
        let command = word(bytes, 0);
        if command != 1 && command != 2 {
            return Err(Error::Malformed);
        }
        if word(bytes, 8) != expected_device {
            return Err(Error::WrongDevice);
        }
        let direction = match word(bytes, 12) {
            0 => Direction::Out,
            1 => Direction::In,
            _ => return Err(Error::Malformed),
        };
        let endpoint = u8::try_from(word(bytes, 16)).map_err(|_| Error::Malformed)?;
        if endpoint > 15 {
            return Err(Error::Malformed);
        }
        let mut header = Self {
            sequence: word(bytes, 4),
            device: expected_device,
            endpoint,
            direction,
            transfer_flags: 0,
            transfer_bytes: 0,
            start_frame: 0,
            interval: 0,
            setup: [0; 8],
            packets: 0,
            unlink: None,
        };
        if command == 2 {
            if endpoint != 0 || bytes[24..48].iter().any(|b| *b != 0) {
                return Err(Error::Malformed);
            }
            header.unlink = Some(word(bytes, 20));
            return Ok(header);
        }
        header.transfer_flags = word(bytes, 20);
        header.transfer_bytes = usize::try_from(word(bytes, 24)).map_err(|_| Error::Limit)?;
        header.start_frame = word(bytes, 28);
        let count = word(bytes, 32);
        // Kernel revisions use zero; the documented non-ISO sentinel is -1.
        header.packets = if count == u32::MAX {
            0
        } else {
            usize::try_from(count).map_err(|_| Error::Limit)?
        };
        if header.transfer_bytes > MAX_TRANSFER_BYTES || header.packets > MAX_ISO_PACKETS {
            return Err(Error::Limit);
        }
        header.interval = word(bytes, 36);
        header.setup.copy_from_slice(&bytes[40..48]);
        if endpoint == 0 && header.packets != 0 {
            return Err(Error::Malformed);
        }
        Ok(header)
    }
    #[must_use]
    pub fn wire_bytes(self) -> usize {
        HEADER_BYTES
            + if self.direction == Direction::Out {
                self.transfer_bytes
            } else {
                0
            }
            + self.packets * 16
    }
    #[must_use]
    pub const fn packets(self) -> usize {
        self.packets
    }
    #[must_use]
    pub const fn unlink_sequence(self) -> Option<u32> {
        self.unlink
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IsoPacket {
    pub offset: u32,
    pub length: u32,
}
#[derive(Debug)]
pub struct Submit<'a> {
    header: Header,
    payload: &'a [u8],
    packets: &'a [u8],
}
#[derive(Debug)]
pub enum Request<'a> {
    Submit(Submit<'a>),
    Unlink { sequence: u32, target: u32 },
}
impl Submit<'_> {
    #[must_use]
    pub const fn header(&self) -> Header {
        self.header
    }
    #[must_use]
    pub const fn payload(&self) -> &[u8] {
        self.payload
    }
    pub fn packets(&self) -> impl Iterator<Item = IsoPacket> + '_ {
        self.packets.chunks_exact(16).map(|p| IsoPacket {
            offset: word(p, 0),
            length: word(p, 4),
        })
    }
    /// Encode a completion after the device personality has accepted/rejected it.
    /// For IN, data is packed without ISO padding. For OUT, no payload is returned.
    /// ISO actual lengths correspond one-for-one to the submitted packet table.
    pub fn complete(
        &self,
        destination: &mut [u8],
        status: i32,
        data: &[u8],
        actual: usize,
        iso_actual: &[u32],
    ) -> Result<usize, Error> {
        let packets = self.header.packets;
        if actual > self.header.transfer_bytes
            || iso_actual.len() != packets
            || (self.header.direction == Direction::In && data.len() != actual)
            || (self.header.direction == Direction::Out && !data.is_empty())
            || (status != 0 && (actual != 0 || iso_actual.iter().any(|n| *n != 0)))
        {
            return Err(Error::ReplyLength);
        }
        if packets != 0 {
            let mut total = 0_usize;
            for (packet, n) in self.packets().zip(iso_actual) {
                if *n > packet.length {
                    return Err(Error::ReplyLength);
                }
                total = total
                    .checked_add(usize::try_from(*n).map_err(|_| Error::Limit)?)
                    .ok_or(Error::Limit)?;
            }
            if total != actual {
                return Err(Error::ReplyLength);
            }
        }
        let size = HEADER_BYTES + data.len() + packets * 16;
        if destination.len() < size {
            return Err(Error::Incomplete);
        }
        destination[..size].fill(0);
        put(destination, 0, 3);
        put(destination, 4, self.header.sequence);
        destination[20..24].copy_from_slice(&status.to_be_bytes());
        put(
            destination,
            24,
            u32::try_from(actual).map_err(|_| Error::Limit)?,
        );
        put(destination, 28, self.header.start_frame);
        put(
            destination,
            32,
            u32::try_from(packets).map_err(|_| Error::Limit)?,
        );
        if status != 0 {
            put(
                destination,
                36,
                u32::try_from(packets).map_err(|_| Error::Limit)?,
            );
        }
        destination[HEADER_BYTES..HEADER_BYTES + data.len()].copy_from_slice(data);
        let mut offset = HEADER_BYTES + data.len();
        for (packet, n) in self.packets().zip(iso_actual) {
            put(destination, offset, packet.offset);
            put(destination, offset + 4, packet.length);
            put(destination, offset + 8, *n);
            destination[offset + 12..offset + 16].copy_from_slice(&status.to_be_bytes());
            offset += 16;
        }
        Ok(size)
    }
}
/// Decode one bounded frame, returning the consumed prefix length.
/// The caller must enforce an absolute read deadline and pending-transfer quota.
pub fn decode(bytes: &[u8], expected_device: u32) -> Result<(Request<'_>, usize), Error> {
    let header = Header::decode(bytes, expected_device)?;
    let length = header.wire_bytes();
    if bytes.len() < length {
        return Err(Error::Incomplete);
    }
    if let Some(target) = header.unlink {
        return Ok((
            Request::Unlink {
                sequence: header.sequence,
                target,
            },
            HEADER_BYTES,
        ));
    }
    let end = HEADER_BYTES
        + if header.direction == Direction::Out {
            header.transfer_bytes
        } else {
            0
        };
    let submit = Submit {
        header,
        payload: &bytes[HEADER_BYTES..end],
        packets: &bytes[end..length],
    };
    let mut previous_end = 0;
    for packet in submit.packets() {
        let end = packet
            .offset
            .checked_add(packet.length)
            .ok_or(Error::Limit)?;
        if packet.offset < previous_end
            || usize::try_from(end).map_err(|_| Error::Limit)? > header.transfer_bytes
        {
            return Err(Error::Malformed);
        }
        previous_end = end;
    }
    Ok((Request::Submit(submit), length))
}
/// A cancelled outstanding URB returns -ECONNRESET; a completed/unknown one returns 0.
pub fn unlink_reply(
    destination: &mut [u8],
    sequence: u32,
    cancelled: bool,
) -> Result<usize, Error> {
    if destination.len() < HEADER_BYTES {
        return Err(Error::Incomplete);
    }
    destination[..HEADER_BYTES].fill(0);
    put(destination, 0, 4);
    put(destination, 4, sequence);
    let status: i32 = if cancelled { -104 } else { 0 };
    destination[20..24].copy_from_slice(&status.to_be_bytes());
    Ok(HEADER_BYTES)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn submit(direction: u32, length: u32, packets: u32) -> Vec<u8> {
        let mut data = vec![0; 48];
        for (offset, value) in [
            (0, 1),
            (4, 7),
            (8, 0x10001),
            (12, direction),
            (16, 1),
            (24, length),
            (32, packets),
        ] {
            put(&mut data, offset, value);
        }
        data
    }
    #[test]
    fn out_control_payload_arrives_before_success_or_stall_completion() {
        let mut data = submit(0, 3, 0);
        put(&mut data, 16, 0);
        data[40..48].copy_from_slice(&[0x21, 9, 2, 3, 0, 0, 3, 0]);
        data.extend([2, 8, 9]);
        let (Request::Submit(request), consumed) = decode(&data, 0x10001).unwrap() else {
            panic!()
        };
        assert_eq!(consumed, 51);
        assert_eq!(request.payload, [2, 8, 9]);
        assert_eq!(request.header.setup, [0x21, 9, 2, 3, 0, 0, 3, 0]);
        let mut reply = [0; 48];
        assert_eq!(request.complete(&mut reply, 0, &[], 3, &[]), Ok(48));
        assert_eq!(word(&reply, 24), 3);
        assert_eq!(request.complete(&mut reply, -32, &[], 0, &[]), Ok(48));
        assert_eq!(&reply[20..24], &(-32_i32).to_be_bytes());
        assert_eq!(&reply[8..20], &[0; 12]);
    }
    #[test]
    fn iso_in_packet_layout_and_packed_reply_are_bounded() {
        let mut data = submit(1, 16, 2);
        for offset in [0_u32, 8] {
            for value in [offset, 4, 0, 0] {
                data.extend(value.to_be_bytes());
            }
        }
        let (Request::Submit(request), _) = decode(&data, 0x10001).unwrap() else {
            panic!()
        };
        assert!(request.payload.is_empty());
        let mut reply = [0; 128];
        assert_eq!(request.complete(&mut reply, 0, &[1; 8], 8, &[4, 4]), Ok(88));
        assert_eq!(word(&reply, 24), 8);
        assert_eq!(word(&reply, 32), 2);
        assert_eq!(word(&reply, 56), 0);
        assert_eq!(word(&reply, 72), 8);
        assert_eq!(word(&reply, 64), 4);
        assert_eq!(word(&reply, 80), 4);
        assert_eq!(
            request.complete(&mut reply, 0, &[1; 8], 8, &[5, 3]),
            Err(Error::ReplyLength)
        );
        assert_eq!(
            request.complete(&mut reply, 0, &[1; 8], 8, &[4]),
            Err(Error::ReplyLength)
        );
        assert_eq!(request.complete(&mut reply, -32, &[], 0, &[0, 0]), Ok(80));
    }
    #[test]
    fn oversized_wrong_device_overlap_and_truncated_frames_fail_closed() {
        assert_eq!(
            Header::decode(&submit(1, 65_537, 0), 0x10001),
            Err(Error::Limit)
        );
        assert_eq!(
            Header::decode(&submit(1, 0, 1025), 0x10001),
            Err(Error::Limit)
        );
        assert_eq!(Header::decode(&submit(1, 0, 0), 2), Err(Error::WrongDevice));
        assert!(matches!(
            decode(&submit(0, 4, 0), 0x10001),
            Err(Error::Incomplete)
        ));
        let mut data = submit(1, 8, 2);
        for offset in [0_u32, 2] {
            for value in [offset, 4, 0, 0] {
                data.extend(value.to_be_bytes());
            }
        }
        assert!(matches!(decode(&data, 0x10001), Err(Error::Malformed)));
        for length in 0..48 {
            assert_eq!(
                Header::decode(&data[..length], 0x10001),
                Err(Error::Incomplete)
            );
        }
    }
    #[test]
    fn unlink_has_no_payload_and_exact_cancel_status() {
        let mut data = submit(0, 0, 0);
        put(&mut data, 0, 2);
        put(&mut data, 16, 0);
        put(&mut data, 20, 6);
        let (Request::Unlink { sequence, target }, consumed) = decode(&data, 0x10001).unwrap()
        else {
            panic!()
        };
        assert_eq!((sequence, target, consumed), (7, 6, 48));
        let mut reply = [0; 48];
        unlink_reply(&mut reply, sequence, true).unwrap();
        assert_eq!(word(&reply, 0), 4);
        assert_eq!(&reply[20..24], &(-104_i32).to_be_bytes());
        unlink_reply(&mut reply, sequence, false).unwrap();
        assert_eq!(word(&reply, 20), 0);
    }
}
