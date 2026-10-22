//! Wii C5 source-only compatibility codec. Every wire table is anchored in
//! wii-c5-report-layouts / wii-c5-memory-errors / wii-c5-synthetic-fixtures.
use super::{WiiRemoteOutputs, WiiRemoteState};
use gr_hid::{
    Command, Delivery, Error, Lifecycle, Protocol, Reply, ReplyError, Report, ReportType,
    RequestKind,
};
use gr_realization_api::RawReverseEvent;
use std::collections::VecDeque;

// wii-c5-report-layouts: payload sizes, excluding the numbered-report byte.
pub(crate) const MODES: &[(u8, usize)] = &[
    (0x30, 2),
    (0x31, 5),
    (0x32, 10),
    (0x33, 17),
    (0x34, 21),
    (0x35, 21),
    (0x36, 21),
    (0x37, 21),
    (0x3d, 21),
    (0x3e, 21),
    (0x3f, 21),
];
const MAX_PENDING: usize = 30;

#[derive(Clone)]
pub(crate) struct WiiProtocol {
    pub(super) state: WiiRemoteState,
    pub(super) outputs: WiiRemoteOutputs,
    mode: u8,
    ir_enabled: [bool; 2],
    pending: VecDeque<Report>,
    next: Option<u64>,
    active: bool,
    valid: bool,
}
impl Default for WiiProtocol {
    fn default() -> Self {
        Self {
            state: WiiRemoteState::default(),
            outputs: WiiRemoteOutputs::default(),
            mode: 0x30,
            ir_enabled: [false; 2],
            pending: VecDeque::new(),
            next: None,
            active: true,
            valid: true,
        }
    }
}
fn input(id: u8, payload: Vec<u8>) -> Report {
    Report::new(ReportType::Input, Some(id), payload).expect("compiled Wii packet is bounded")
}
/// Core bits and embedded raw acceleration are source layout, not normalized axes.
/// Record: wii-c5-report-layouts (Dolphin data report/header; Linux module decoder).
pub(super) fn core(state: &WiiRemoteState) -> [u8; 2] {
    let b = state.buttons;
    [
        u8::from(b[0])
            | (u8::from(b[1]) << 1)
            | (u8::from(b[3]) << 2)
            | (u8::from(b[2]) << 3)
            | (u8::from(b[4]) << 4),
        u8::from(b[7])
            | (u8::from(b[6]) << 1)
            | (u8::from(b[9]) << 2)
            | (u8::from(b[8]) << 3)
            | (u8::from(b[5]) << 4)
            | (u8::from(b[10]) << 7),
    ]
}
#[allow(clippy::cast_possible_truncation)] // Source protocol packs ten-bit coordinates into bytes.
fn basic_ir(state: &WiiRemoteState) -> [u8; 10] {
    let mut bytes = [0xff; 10];
    for (pair, points) in state.ir.chunks_exact(2).enumerate() {
        for (slot, point) in points.iter().enumerate() {
            let (x, y) = point.map_or((1023, 1023), super::WiiIrPoint::coordinates);
            let base = pair * 5;
            let low = if slot == 0 { base } else { base + 3 };
            bytes[low] = x as u8;
            bytes[low + 1] = y as u8;
            let high = ((x >> 8) as u8) | (((y >> 8) as u8) << 2);
            if slot == 0 {
                bytes[base + 2] = (bytes[base + 2] & 0x0f) | (high << 4);
            } else {
                bytes[base + 2] = (bytes[base + 2] & 0xf0) | high;
            }
        }
    }
    bytes
}
/// Encode every legal data mode. Reserved0x38..0x3c fail explicitly. Full-IR
/// point-size/intensity have no physical evidence: only the no-point shape is
/// supported for interleaved modes; visible points are rejected instead of lost.
#[allow(clippy::cast_possible_truncation)]
pub(super) fn data(mode: u8, state: &WiiRemoteState, ir: bool) -> Result<Report, Error> {
    let length = MODES
        .iter()
        .find(|(id, _)| *id == mode)
        .ok_or(Error::InvalidState)?
        .1;
    if matches!(mode, 0x3e | 0x3f) && ir && state.ir.iter().any(Option::is_some) {
        return Err(Error::InvalidState);
    }
    let mut bytes = vec![0; length];
    if mode != 0x3d {
        bytes[..2].copy_from_slice(&core(state));
    }
    let [x, y, z] = state.accelerometer;
    if matches!(mode, 0x31 | 0x33 | 0x35 | 0x37) {
        bytes[0] |= ((x & 3) as u8) << 5;
        bytes[1] |= (((y >> 1) & 1) as u8) << 5 | (((z >> 1) & 1) as u8) << 6;
        bytes[2..5].copy_from_slice(&[(x >> 2) as u8, (y >> 2) as u8, (z >> 2) as u8]);
    } else if mode == 0x3e {
        bytes[0] |= (((z >> 6) & 3) as u8) << 5;
        bytes[1] |= (((z >> 8) & 3) as u8) << 5;
        bytes[2] = (x >> 2) as u8;
    } else if mode == 0x3f {
        bytes[0] |= (((z >> 2) & 3) as u8) << 5;
        bytes[1] |= (((z >> 4) & 3) as u8) << 5;
        bytes[2] = (y >> 2) as u8;
    }
    // wii-c5-camera-no-points / wii-c5-report-layouts.
    match mode {
        0x33 => {
            bytes[5..17].fill(0xff);
            // Point size is not part of the typed point-set contract. Extended
            // visible blobs are deliberately refused until a typed size exists.
            if ir && state.ir.iter().any(Option::is_some) {
                return Err(Error::InvalidState);
            }
        }
        0x36 | 0x37 => {
            let offset = if mode == 0x36 { 2 } else { 5 };
            bytes[offset..offset + 10].copy_from_slice(&if ir {
                basic_ir(state)
            } else {
                [0xff; 10]
            });
        }
        0x3e | 0x3f => bytes[3..].fill(0xff),
        _ => {}
    }
    // No extension present in the codec step. Step13c fills these windows from
    // typed accessories; absent windows use the synthetic no-accessory fixture.
    let extension = match mode {
        0x32 => Some((2, 8)),
        0x34 => Some((2, 19)),
        0x35 => Some((5, 16)),
        0x36 => Some((12, 9)),
        0x37 => Some((15, 6)),
        0x3d => Some((0, 21)),
        _ => None,
    };
    if let Some((offset, size)) = extension {
        bytes[offset..offset + size].fill(0xff);
    }
    Ok(input(mode, bytes))
}
impl WiiProtocol {
    pub(super) fn status(&self) -> Report {
        // wii-c5-report-layouts / wii-kernel-outputs-battery. Raw battery byte;
        // no invented conversion from an estimated charge percentage.
        let buttons = core(&self.state);
        let leds = self
            .outputs
            .leds
            .iter()
            .enumerate()
            .fold(0, |bits, (i, on)| bits | (u8::from(*on) << (i + 4)));
        let flags =
            u8::from(self.state.battery < 0x20) | (u8::from(self.ir_enabled[0]) << 3) | leds;
        input(
            0x20,
            vec![buttons[0], buttons[1], flags, 0, 0, self.state.battery],
        )
    }
    pub(super) fn ack(&self, id: u8, code: u8) -> Report {
        // wii-c5-memory-errors / wii-c5-report-layouts.
        let b = core(&self.state);
        input(0x22, vec![b[0], b[1], id, code])
    }
    fn memory_error(&self, address: [u8; 2], code: u8) -> Report {
        let b = core(&self.state);
        let mut bytes = vec![b[0], b[1], 0xf0 | code, address[0], address[1]];
        bytes.resize(21, 0);
        input(0x21, bytes)
    }
    fn accept(&mut self, report: &Report, now: u64) -> Result<(), ReplyError> {
        // Transactional admission: malformed input cannot change retained state,
        // queue replies, change modes or increment the accepted-output revision.
        let mut next = self.clone();
        next.accept_inner(report, now)?;
        *self = next;
        Ok(())
    }
    fn accept_inner(&mut self, report: &Report, now: u64) -> Result<(), ReplyError> {
        if !self.active || !self.valid {
            return Err(ReplyError::Unsupported);
        }
        if report.kind != ReportType::Output {
            return Err(ReplyError::Unsupported);
        }
        let id = report.id().ok_or(ReplyError::Unsupported)?;
        let p = report.payload();
        let size = match id {
            0x10 | 0x11 | 0x13 | 0x15 | 0x1a => 1,
            0x12 => 2,
            0x16 => 21,
            0x17 => 6,
            _ => return Err(ReplyError::Unsupported),
        };
        if p.len() != size {
            return Err(ReplyError::Invalid);
        }
        if self.pending.len() >= MAX_PENDING {
            return Err(ReplyError::Busy);
        }
        let before = self.outputs;
        match id {
            0x10 => {}
            0x11 => self.outputs.leds = std::array::from_fn(|i| p[0] & (1 << (i + 4)) != 0),
            0x12 => {
                if !MODES.iter().any(|(mode, _)| *mode == p[1]) {
                    return Err(ReplyError::Invalid);
                }
                // Do not accept a mode that would silently drop visible points.
                data(p[1], &self.state, self.ir_enabled[0]).map_err(|_| ReplyError::Invalid)?;
                self.mode = p[1];
            }
            0x13 => self.ir_enabled[0] = p[0] & 4 != 0,
            0x1a => self.ir_enabled[1] = p[0] & 4 != 0,
            0x15 => {
                self.outputs.rumble = p[0] & 1 != 0;
                self.pending.push_back(self.status());
            }
            0x16 => {
                if p[4] == 0 || p[4] > 16 {
                    return Err(ReplyError::Invalid);
                }
                let code = match (p[0] >> 2) & 3 {
                    0 => 8,
                    1 | 2 if p[1] >> 1 == 0x50 => 8,
                    1 | 2 => 7,
                    _ => 6,
                };
                self.pending.push_back(self.ack(id, code));
            }
            0x17 => {
                if p[4] != 0 || p[5] != 0 {
                    let code = match (p[0] >> 2) & 3 {
                        0 => 8,
                        1 | 2 if p[1] >> 1 == 0x50 => 8,
                        1 | 2 => 7,
                        _ => 6,
                    };
                    self.pending
                        .push_back(self.memory_error([p[2], p[3]], code));
                }
            }
            _ => unreachable!(),
        }
        self.outputs.rumble = p[0] & 1 != 0;
        if self.outputs.leds != before.leds || self.outputs.rumble != before.rumble {
            self.outputs.revision = before.revision.checked_add(1).ok_or(ReplyError::Busy)?;
        }
        if matches!(id, 0x11 | 0x12 | 0x13 | 0x1a) && p[0] & 2 != 0 {
            self.pending.push_back(self.ack(id, 0));
        }
        self.next = Some(now);
        Ok(())
    }
    fn observe(report: &Report) -> RawReverseEvent {
        RawReverseEvent::HidOutput {
            report_id: report.id(),
            bytes: report.payload().to_vec(),
        }
    }
}
impl Protocol for WiiProtocol {
    type State = WiiRemoteState;
    type Output = RawReverseEvent;
    fn neutral(&self) -> Self::State {
        WiiRemoteState::new(self.state.revision())
    }
    fn validate(&self, state: &Self::State) -> Result<(), Error> {
        if !self.valid
            || !state.revision().has_identity_evidence()
            || state.accelerometer.iter().any(|v| *v > 1023)
        {
            return Err(Error::InvalidState);
        }
        data(self.mode, state, self.ir_enabled[0]).map(|_| ())
    }
    fn input(&mut self, state: &Self::State, _: u64) -> Result<Vec<Report>, Error> {
        self.validate(state)?;
        self.state = state.clone();
        if !self.active {
            return Ok(Vec::new());
        }
        let mut reports: Vec<_> = self.pending.drain(..).collect();
        reports.push(data(self.mode, state, self.ir_enabled[0])?);
        if self.mode == 0x3e {
            self.mode = 0x3f;
        } else if self.mode == 0x3f {
            self.mode = 0x3e;
        }
        // Cadence is not measured: no fabricated firmware interval is advertised.
        // Application edits/commits and protocol requests drive reports.
        self.next = None;
        Ok(reports)
    }
    fn deadline(&self) -> Option<u64> {
        self.next
    }
    fn request(&mut self, kind: &RequestKind, now: u64) -> (Reply, Option<RawReverseEvent>) {
        match kind {
            RequestKind::Get { .. } => (Reply::Get(Err(ReplyError::Unsupported)), None),
            RequestKind::Set(report) => {
                let result = self.accept(report, now);
                let observation = result.is_ok().then(|| Self::observe(report));
                (Reply::Set(result), observation)
            }
        }
    }
    fn output(&mut self, report: Report, now: u64) -> Result<Option<RawReverseEvent>, Error> {
        match self.accept(&report, now) {
            Ok(()) => Ok(Some(Self::observe(&report))),
            Err(ReplyError::Busy) => Err(Error::Transport),
            Err(ReplyError::Invalid | ReplyError::Unsupported) => Ok(None),
        }
    }
    fn lifecycle(&mut self, event: Lifecycle, now: u64) -> Option<RawReverseEvent> {
        match event {
            Lifecycle::Start {
                numbered_input,
                numbered_output,
                numbered_feature,
            } => {
                self.valid = numbered_input && numbered_output && !numbered_feature;
                self.active = true;
                self.next = Some(now);
            }
            Lifecycle::Stop => {
                self.active = false;
                self.pending.clear();
                self.next = None;
            }
            Lifecycle::Open | Lifecycle::Close => {}
        }
        Some(RawReverseEvent::HidLifecycle(event))
    }
    fn delivered(&mut self, _: &Command, _: Delivery) {}
}

#[cfg(test)]
mod tests;
