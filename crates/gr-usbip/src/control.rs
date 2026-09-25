//! Standard USB/UAC2 controls and forwarding into controller-owned HID semantics.
use crate::profile::{Profile, ProfileId};
use gr_hid::{Report, ReportType, RequestKind};
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Completion {
    Data(Vec<u8>),
    Hid(RequestKind),
    Stall,
}
#[derive(Debug, Default)]
pub struct ControlState {
    configuration: u8,
    alternate: [u8; 4],
}
impl ControlState {
    #[must_use]
    pub const fn configured(&self) -> bool {
        self.configuration == 1
    }
    #[must_use]
    pub fn playback_active(&self) -> bool {
        self.configured() && self.alternate[1] == 1
    }
    #[must_use]
    pub fn microphone_active(&self) -> bool {
        self.configured() && self.alternate[2] == 1
    }
    /// Complete metadata is available before the OUT payload is interpreted.
    /// The worker must send the returned result exactly once through USB/IP.
    pub fn handle(&mut self, profile: &Profile, setup: [u8; 8], payload: &[u8]) -> Completion {
        let value = u16::from_le_bytes([setup[2], setup[3]]);
        let index = u16::from_le_bytes([setup[4], setup[5]]);
        let length = usize::from(u16::from_le_bytes([setup[6], setup[7]]));
        let input = setup[0] & 0x80 != 0;
        if (input && !payload.is_empty()) || (!input && payload.len() != length) {
            return Completion::Stall;
        }
        let result = match (setup[0], setup[1]) {
            (0x80, 6) => descriptor(profile, value, index),
            (0x81, 6) if index == 3 => match value {
                0x2200 => Some(profile.report().to_vec()),
                0x2100 => {
                    let Ok(size) = u16::try_from(profile.report().len()) else {
                        return Completion::Stall;
                    };
                    let bytes = size.to_le_bytes();
                    Some(vec![9, 0x21, 0x11, 1, 0, 1, 0x22, bytes[0], bytes[1]])
                }
                _ => None,
            },
            (0x80, 8) if value == 0 && index == 0 && length == 1 => Some(vec![self.configuration]),
            (0x00, 9) if value <= 1 && index == 0 && length == 0 => {
                self.configuration = value.to_le_bytes()[0];
                self.alternate.fill(0);
                Some(vec![])
            }
            (0x81, 10) if self.configured() && index < 4 && value == 0 && length == 1 => {
                Some(vec![self.alternate[usize::from(index)]])
            }
            (0x01, 11) if self.configured() && index < 4 && length == 0 => {
                let valid = value == 0 || ((index == 1 || index == 2) && value == 1);
                if valid {
                    self.alternate[usize::from(index)] = value.to_le_bytes()[0];
                    Some(vec![])
                } else {
                    None
                }
            }
            (0x80, 0) if value == 0 && index == 0 && length == 2 => Some(vec![0, 0]),
            (0x81, 0) if self.configured() && value == 0 && index < 4 && length == 2 => {
                Some(vec![0, 0])
            }
            (0xa1, 1 | 2) if self.configured() && index == 0x1000 => clock(setup[1], value),
            (0xa1, 1) if self.configured() && index == 3 && (1..=4096).contains(&length) => {
                let (kind, id) = report_type(value);
                if (profile.id() != ProfileId::Xbox360HidEmulated) != id.is_some() {
                    return Completion::Stall;
                }
                return Completion::Hid(RequestKind::Get { kind, id });
            }
            (0x21, 9) if self.configured() && index == 3 && (1..=4096).contains(&length) => {
                let (kind, id) = report_type(value);
                let numbered = profile.id() != ProfileId::Xbox360HidEmulated;
                if numbered != id.is_some() {
                    return Completion::Stall;
                }
                let Ok(report) = Report::from_wire(kind, numbered, payload) else {
                    return Completion::Stall;
                };
                if report.id() != id {
                    return Completion::Stall;
                }
                return Completion::Hid(RequestKind::Set(report));
            }
            _ => None,
        };
        result.map_or(Completion::Stall, |mut data| {
            data.truncate(length);
            Completion::Data(data)
        })
    }
}
fn report_type(value: u16) -> (ReportType, Option<u8>) {
    let [id, kind] = value.to_le_bytes();
    let kind = match kind {
        1 => ReportType::Input,
        2 => ReportType::Output,
        3 => ReportType::Feature,
        other => ReportType::Other(other),
    };
    (kind, (id != 0).then_some(id))
}
fn clock(request: u8, value: u16) -> Option<Vec<u8>> {
    match (request, value) {
        (1, 0x0100) => Some(48_000_u32.to_le_bytes().to_vec()),
        (1, 0x0200) => Some(vec![1]),
        (2, 0x0100) => {
            let mut result = vec![1, 0]; // One subrange: fixed 48000 Hz.
            for value in [48_000_u32, 48_000, 0] {
                result.extend(value.to_le_bytes());
            }
            Some(result)
        }
        _ => None,
    }
}
fn descriptor(profile: &Profile, value: u16, language: u16) -> Option<Vec<u8>> {
    let [index, kind] = value.to_le_bytes();
    match (kind, index) {
        (1, 0) if language == 0 => Some(profile.device().to_vec()),
        (2, 0) if language == 0 => Some(profile.configuration().to_vec()),
        (3, _) if language == 0 || language == 0x0409 => profile.string(index),
        (6, 0) if language == 0 => Some(vec![10, 6, 0, 2, 0xef, 2, 1, 64, 1, 0]),
        (7, 0) if language == 0 => {
            let mut data = profile.configuration().to_vec();
            data[1] = 7;
            let mut offset = 0;
            while offset < data.len() {
                if data[offset + 1] == 5 {
                    data[offset + 6] = 1;
                }
                offset += usize::from(data[offset]);
            }
            Some(data)
        }
        _ => None,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn configured() -> (Profile, ControlState) {
        let profile = Profile::new(ProfileId::DualSenseEmulated);
        let mut state = ControlState::default();
        assert_eq!(
            state.handle(&profile, [0, 9, 1, 0, 0, 0, 0, 0], &[]),
            Completion::Data(vec![])
        );
        (profile, state)
    }
    #[test]
    fn enumeration_clock_ranges_and_alternate_setting_lifecycle() {
        let (p, mut s) = configured();
        assert_eq!(
            s.handle(&p, [0x80, 6, 0, 1, 0, 0, 8, 0], &[]),
            Completion::Data(p.device()[..8].to_vec())
        );
        assert_eq!(
            s.handle(&p, [0xa1, 1, 0, 1, 0, 0x10, 4, 0], &[]),
            Completion::Data(48_000_u32.to_le_bytes().to_vec())
        );
        assert_eq!(
            s.handle(&p, [0xa1, 2, 0, 1, 0, 0x10, 2, 0], &[]),
            Completion::Data(vec![1, 0])
        );
        assert_eq!(
            s.handle(&p, [0x21, 1, 0, 1, 0, 0x10, 4, 0], &[0; 4]),
            Completion::Stall
        );
        assert_eq!(
            s.handle(&p, [1, 11, 1, 0, 1, 0, 0, 0], &[]),
            Completion::Data(vec![])
        );
        assert!(s.playback_active());
        assert!(!s.microphone_active());
        assert_eq!(
            s.handle(&p, [1, 11, 2, 0, 1, 0, 0, 0], &[]),
            Completion::Stall
        );
        assert!(s.playback_active());
        assert_eq!(
            s.handle(&p, [0, 9, 0, 0, 0, 0, 0, 0], &[]),
            Completion::Data(vec![])
        );
        assert!(!s.playback_active());
        assert!(!s.configured());
    }
    #[test]
    fn hid_report_classes_and_payload_validation_remain_controller_owned() {
        let (p, mut s) = configured();
        for (tag, kind) in [
            (1, ReportType::Input),
            (2, ReportType::Output),
            (3, ReportType::Feature),
            (7, ReportType::Other(7)),
        ] {
            assert_eq!(
                s.handle(&p, [0xa1, 1, 2, tag, 3, 0, 64, 0], &[]),
                Completion::Hid(RequestKind::Get { kind, id: Some(2) })
            );
            assert_eq!(
                s.handle(&p, [0x21, 9, 2, tag, 3, 0, 3, 0], &[2, 4, 5]),
                Completion::Hid(RequestKind::Set(
                    Report::new(kind, Some(2), vec![4, 5]).unwrap()
                ))
            );
            assert_eq!(
                s.handle(&p, [0x21, 9, 2, tag, 3, 0, 3, 0], &[3, 4, 5]),
                Completion::Stall
            );
            assert_eq!(
                s.handle(&p, [0x21, 9, 2, tag, 3, 0, 3, 0], &[2, 4]),
                Completion::Stall
            );
        }
    }
}
