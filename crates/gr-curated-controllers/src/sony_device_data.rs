//! Runtime-only Sony feature-report validation and caller-chosen file format.
//! Files are deliberately excluded from repository fixtures and contain no identity.
use std::{fs, io, path::Path};

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

pub(crate) fn validate<const N: usize>(
    id: u8,
    bytes: &[u8],
    expected_id: u8,
) -> io::Result<[u8; N]> {
    if id != expected_id || bytes.len() != N || bytes.first() != Some(&expected_id) {
        return Err(invalid("invalid Sony feature report ID or exact length"));
    }
    bytes
        .try_into()
        .map_err(|_| invalid("invalid Sony report length"))
}

pub(crate) fn validate_motion(bytes: &[u8]) -> io::Result<()> {
    let sample = |index: usize| i16::from_le_bytes([bytes[index], bytes[index + 1]]);
    let speed = i32::from(sample(19)) + i32::from(sample(21));
    if speed == 0 {
        return Err(invalid("motion calibration gyro speed denominator is zero"));
    }
    for (high, low) in [(7, 9), (11, 13), (15, 17), (23, 25), (27, 29), (31, 33)] {
        if i32::from(sample(high)) - i32::from(sample(low)) == 0 {
            return Err(invalid("motion calibration axis denominator is zero"));
        }
    }
    Ok(())
}

fn encode_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut result = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        write!(&mut result, "{b:02x}").expect("format into String");
    }
    result
}
fn decode_hex(value: &str, max_bytes: usize) -> io::Result<Vec<u8>> {
    if value.len() % 2 != 0 || value.len() > max_bytes * 2 || !value.is_ascii() {
        return Err(invalid("invalid Sony device-data hex length"));
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let token = std::str::from_utf8(pair).map_err(|_| invalid("invalid report hex"))?;
            u8::from_str_radix(token, 16).map_err(|_| invalid("invalid report hex"))
        })
        .collect()
}

/// Serialize only the two allowed reports; never serialize pairing/identity.
pub(crate) fn save(path: impl AsRef<Path>, calibration: &[u8], firmware: &[u8]) -> io::Result<()> {
    let encoded = format!(
        "vg-sony-device-data-v1\n{:02x} {}\n{:02x} {}\n",
        calibration[0],
        encode_hex(calibration),
        firmware[0],
        encode_hex(firmware),
    );
    fs::write(path, encoded)
}

/// Strict, bounded, atomic parse. A failed parse never returns partial device data.
pub(crate) fn load(
    path: impl AsRef<Path>,
    calibration_id: u8,
    firmware_id: u8,
    calibration_len: usize,
    firmware_len: usize,
) -> io::Result<(Vec<u8>, Vec<u8>)> {
    let path = path.as_ref();
    let max_size = 80 + (calibration_len + firmware_len) * 2;
    if fs::metadata(path)?.len() > max_size as u64 {
        return Err(invalid("Sony device-data file exceeds configured limit"));
    }
    let file = fs::read_to_string(path)?;
    let mut lines = file.lines();
    if lines.next() != Some("vg-sony-device-data-v1") {
        return Err(invalid("unsupported Sony device-data file version"));
    }
    let mut report = |id: u8, size: usize| -> io::Result<Vec<u8>> {
        let line = lines
            .next()
            .ok_or_else(|| invalid("missing Sony feature report"))?;
        let (tag, hex) = line
            .split_once(' ')
            .ok_or_else(|| invalid("invalid Sony report row"))?;
        if tag != format!("{id:02x}") {
            return Err(invalid("Sony feature reports out of order or wrong family"));
        }
        let bytes = decode_hex(hex, size)?;
        if bytes.len() != size || bytes.first() != Some(&id) {
            return Err(invalid("Sony feature report has wrong length or ID"));
        }
        Ok(bytes)
    };
    let calibration = report(calibration_id, calibration_len)?;
    let firmware = report(firmware_id, firmware_len)?;
    if lines.next().is_some() || !file.ends_with('\n') {
        return Err(invalid("unexpected trailing Sony device-data content"));
    }
    Ok((calibration, firmware))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn synthetic_file_format_rejects_extra_report_and_odd_hex() {
        assert!(decode_hex("a", 1).is_err());
        assert!(decode_hex("gh", 1).is_err());
        assert_eq!(decode_hex("04ff", 2).unwrap(), vec![4, 255]);
        let path = std::env::temp_dir().join(format!(
            "vg-sony-device-data-test-{}-{}.txt",
            std::process::id(),
            "strict"
        ));
        fs::write(&path, "vg-sony-device-data-v1\n05 05\n20 20\n09 09\n").unwrap();
        assert!(load(&path, 5, 0x20, 1, 1).is_err());
        fs::remove_file(path).unwrap();
    }
}
