//! Compat only: steam-c6-input-42/45/47, steam-c6-feature-kernel-exchange,
//! steam-c6-output-80-81 and steam-c6-output-82-85. No device ACKs or timers.
use super::{Battery, SteamController2026State as State};
use gr_hid::{
    Command, Delivery, Error, Lifecycle, Protocol, Reply, ReplyError, Report, ReportType,
    RequestKind,
};
use gr_realization_api::ProviderFrame;
use gr_realization_api::RawReverseEvent;

// steam-d9-1302-model-attributes / steam-d9-feature-response-framing:
// exact non-identifying 0x83 reply, firmware build 0x6ACFF323, HW_ID 0x48.
const STEAM_CLIENT_ATTRIBUTES: &[u8] = &include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/protocol-research/steam-d9-1302-attributes.rs"
));
// steam-d9-steam-setting-acceptance: observed transport acceptance only.
// Retention models the last accepted intent, not device readback or effects.
const STEAM_CLIENT_SETTING_KEYS: &[u8] =
    &[7, 8, 9, 24, 34, 35, 45, 46, 48, 49, 50, 52, 53, 82, 84, 85];

// steam-c6-input-42/45/47: lengths INCLUDE the numbered report byte.
// steam-s3-1302-report-shapes: declared lengths are faithful (fresh physical capture).
// Field semantics remain compat consumer interpretations.
#[derive(Debug, Clone, Copy)]
enum Format {
    Primary42,
    Alternate45,
    Sdl47 { pad_timestamp: u16, imu_ticks: u16 },
}

fn input(state: &State, sequence: u8, format: Format) -> Report {
    let (id, length) = match format {
        Format::Primary42 => (0x42, 54),
        Format::Alternate45 => (0x45, 46),
        Format::Sdl47 { .. } => (0x47, 46),
    };
    let mut wire = vec![0; length];
    wire[0] = id;
    wire[1] = sequence;
    // steam-c6-input-buttons, steam-c6-input-scaling: LE native raw values.
    wire[2..6].copy_from_slice(&state.button_bits().to_le_bytes());
    for (offset, value) in [
        (6, state.triggers[0]),
        (8, state.triggers[1]),
        (10, state.sticks[0][0]),
        (12, state.sticks[0][1]),
        (14, state.sticks[1][0]),
        (16, state.sticks[1][1]),
    ] {
        wire[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }
    let shift = if let Format::Sdl47 {
        pad_timestamp,
        imu_ticks,
    } = format
    {
        wire[18..20].copy_from_slice(&pad_timestamp.to_le_bytes());
        wire[32..34].copy_from_slice(&imu_ticks.to_le_bytes());
        2
    } else {
        wire[30..34].copy_from_slice(&state.motion.timestamp_us.to_le_bytes());
        0
    };
    for (pad, base) in state.pads.iter().zip([18 + shift, 24 + shift]) {
        wire[base..base + 2].copy_from_slice(&pad.position[0].to_le_bytes());
        wire[base + 2..base + 4].copy_from_slice(&pad.position[1].to_le_bytes());
        wire[base + 4..base + 6].copy_from_slice(&pad.pressure.to_le_bytes());
    }
    for (offset, values) in [
        (34, &state.motion.accelerometer[..]),
        (40, &state.motion.gyroscope[..]),
    ] {
        for (slot, value) in wire[offset..offset + 6].chunks_exact_mut(2).zip(values) {
            slot.copy_from_slice(&value.to_le_bytes());
        }
    }
    if matches!(format, Format::Primary42) {
        for (slot, value) in wire[46..54]
            .chunks_exact_mut(2)
            .zip(state.motion.quaternion)
        {
            slot.copy_from_slice(&value.to_le_bytes());
        }
    }
    Report::from_wire(ReportType::Input, true, &wire).expect("compiled Steam frame")
}

pub(super) fn provider_frame(state: &State) -> ProviderFrame {
    let report = input(state, 0, Format::Primary42);
    ProviderFrame::HidInput {
        report_id: report.id(),
        bytes: report.payload().to_vec(),
    }
}

// steam-s3-1302-sdl47-descriptor-conflict: never submit the diagnostic-only
// SDL alternate through a wired path described by the captured bytes.
fn wired_input(state: &State, sequence: u8, format: Format) -> Result<Report, Error> {
    if matches!(format, Format::Sdl47 { .. }) {
        return Err(Error::InvalidState);
    }
    Ok(input(state, sequence, format))
}

fn battery(state: &Battery) -> Report {
    // steam-c6-battery-43: 15-byte layout, no asserted physical values/units.
    let mut wire = vec![0x43, state.state, state.level];
    for value in [
        state.voltage,
        state.system_voltage,
        state.input_voltage,
        state.current,
        state.input_current,
        state.temperature,
    ] {
        wire.extend(value.to_le_bytes());
    }
    Report::from_wire(ReportType::Input, true, &wire).expect("compiled battery frame")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SteamController2026Haptic {
    // steam-c6-output-80-81 / disagreement-gain-kernel/sdl: gains stay raw.
    Rumble {
        kind: u8,
        intensity: u16,
        left_speed: u16,
        left_gain: u8,
        right_speed: u16,
        right_gain: u8,
    },
    Pulse {
        side: u8,
        on_us: u16,
        off_us: u16,
        count: u16,
    },
    // steam-c6-output-82-85: source declarations, not actuator validation.
    Command {
        side: u8,
        command: u8,
        gain_db: i8,
    },
    Lfo {
        side: u8,
        gain_db: i8,
        frequency: u16,
        duration_ms: u16,
        lfo_frequency: u16,
        depth: u8,
    },
    Sweep {
        side: u8,
        gain_db: i8,
        duration_ms: u16,
        start: u16,
        end: u16,
    },
    Script {
        side: u8,
        script: u8,
        gain_db: i8,
    },
}
fn le(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}
fn signed(byte: u8) -> i8 {
    i8::from_le_bytes([byte])
}
impl SteamController2026Haptic {
    fn decode(report: &Report) -> Result<Self, ReplyError> {
        if report.kind != ReportType::Output {
            return Err(ReplyError::Unsupported);
        }
        let b = report.payload();
        // steam-c6-output-80-81 and steam-c6-output-82-85: payload sizes exclude ID.
        // sc2-july-wired-descriptor-report-shapes: physical (historical capture,
        // raw not revalidated) BODY lengths only; semantics remain compat.
        Ok(match (report.id(), b.len()) {
            (Some(0x80), 9) => Self::Rumble {
                kind: b[0],
                intensity: le(b, 1),
                left_speed: le(b, 3),
                left_gain: b[5],
                right_speed: le(b, 6),
                right_gain: b[8],
            },
            (Some(0x81), 7) => Self::Pulse {
                side: b[0],
                on_us: le(b, 1),
                off_us: le(b, 3),
                count: le(b, 5),
            },
            (Some(0x82), 3) => Self::Command {
                side: b[0],
                command: b[1],
                gain_db: signed(b[2]),
            },
            (Some(0x83), 9) => Self::Lfo {
                side: b[0],
                gain_db: signed(b[1]),
                frequency: le(b, 2),
                duration_ms: le(b, 4),
                lfo_frequency: le(b, 6),
                depth: b[8],
            },
            (Some(0x84), 8) => Self::Sweep {
                side: b[0],
                gain_db: signed(b[1]),
                duration_ms: le(b, 2),
                start: le(b, 4),
                end: le(b, 6),
            },
            (Some(0x85), 3) => Self::Script {
                side: b[0],
                script: b[1],
                gain_db: signed(b[2]),
            },
            (Some(0x80..=0x85), _) => return Err(ReplyError::Invalid),
            _ => return Err(ReplyError::Unsupported),
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Outputs {
    revision: u64,
    pub haptics: [Option<SteamController2026Haptic>; 6],
    // steam-c6-feature-kernel-lizard / feature-sdl-lizard / feature-imu.
    // Unknown defaults remain None, not invented retail settings.
    mappings: Option<bool>,
    lizard: Option<bool>,
    watchdog: Option<u16>,
    imu: Option<u16>,
    compat_settings: std::collections::BTreeMap<u8, u16>,
}
#[derive(Debug, Clone, Default)]
struct SteamProtocol {
    serial: String,
    steam_client_compat: bool,
    sequence: u8,
    outputs: Outputs,
    query: Option<u8>,
    stopped: bool,
    invalid_framing: bool,
    refresh: Option<u64>,
}
impl SteamProtocol {
    fn feature(&mut self, report: &Report) -> Result<(), ReplyError> {
        // steam-c6-no-wired-feature2: do not serve receiver feature 2 on wired.
        if report.kind != ReportType::Feature || report.id() != Some(1) {
            return Err(ReplyError::Unsupported);
        }
        // Any new SET supersedes the preceding exchange, including invalid SETs.
        self.query = None;
        // steam-c6-feature-kernel-exchange: fixed 64-byte numbered host buffer.
        let b = report.payload();
        if b.len() != 63 {
            return Err(ReplyError::Invalid);
        }
        let mut next = self.outputs.clone();
        match b[0] {
            // hid-steam v7.3-rc1, lines 667–685: current request and documented historical length.
            0xae if matches!(b[1], 0x15 | 0x16) && b[2] == 1 => self.query = Some(0xae),
            0x83 if b[1] == 0 => self.query = Some(0x83),
            // steam-d9-feature-response-framing: these exact two-byte SET
            // bodies completed successfully. No semantics or retained state
            // are inferred from that transport acknowledgement.
            0xdc if self.steam_client_compat && b[1..4] == [2, 1, 2] => {}
            0xe2 if self.steam_client_compat && b[1..4] == [2, 1, 0x20] => {}
            // steam-d9-feature-response-framing: only the first ED exchange
            // (SET length 24, GET length 1, value 0) has a reviewed body.
            // Do not retain opaque request bytes or replay opaque reply tails.
            0xed if self.steam_client_compat && b[1] == 24 => self.query = Some(0xed),
            // steam-c6-feature-kernel-lizard: mapping and settings are distinct.
            0x81 if b[1] == 0 => next.mappings = Some(false),
            0x85 if b[1] == 0 => next.mappings = Some(true),
            0x8e if b[1] == 0 => {
                // Consumer restore-default intent; retail defaults unknown.
                next.lizard = None;
                next.imu = None;
                next.watchdog = None;
                next.compat_settings.clear();
            }
            0x87 => {
                // steam-c6-feature-kernel-lizard / feature-sdl-lizard / feature-imu.
                let size = usize::from(b[1]);
                if size == 0 || size % 3 != 0 || size > b.len() - 2 {
                    return Err(ReplyError::Invalid);
                }
                for setting in b[2..2 + size].chunks_exact(3) {
                    let value = le(setting, 1);
                    match setting[0] {
                        9 if value <= 1 => next.lizard = Some(value != 0),
                        9 => return Err(ReplyError::Invalid),
                        48 => next.imu = Some(value),
                        71 => next.watchdog = Some(value),
                        key if self.steam_client_compat
                            && STEAM_CLIENT_SETTING_KEYS.contains(&key) => {}
                        // Unreviewed keys remain atomic failures in both profiles.
                        _ => return Err(ReplyError::Unsupported),
                    }
                    if self.steam_client_compat {
                        next.compat_settings.insert(setting[0], value);
                    }
                }
            }
            0xae | 0x83 | 0x81 | 0x85 | 0x8e => return Err(ReplyError::Invalid),
            _ => {
                // steam-c6-feature-kernel-exchange / reply-attributes-short:
                // reject SET, and provide a deliberately malformed exchange reply
                // if a host still GETs it. Kernel detects -EPROTO, not stale data.
                // This fits the kernel error path; it is not device behaviour.
                self.query = Some(b[0]);
                return Err(ReplyError::Unsupported);
            }
        }
        if next != self.outputs {
            next.revision = self.outputs.revision.wrapping_add(1);
            self.outputs = next;
        }
        Ok(())
    }
    fn get_feature(&self) -> Result<Report, ReplyError> {
        if self.steam_client_compat && self.query == Some(0x83) {
            return Ok(
                Report::from_wire(ReportType::Feature, true, STEAM_CLIENT_ATTRIBUTES)
                    .expect("compiled physical model attribute reply"),
            );
        }
        // steam-c6-feature-registration / reply-serial-empty / reply-attributes-empty.
        // The serial is virtual instance identity. Default attributes are structural.
        let mut wire = vec![0; 64];
        wire[0] = 1;
        match self.query {
            Some(0xed) if self.steam_client_compat => {
                wire[1..4].copy_from_slice(&[0xed, 1, 0]);
                wire.truncate(63);
            }
            Some(0xae) => {
                let serial = self.serial.as_bytes();
                wire[1..4].copy_from_slice(&[
                    0xae,
                    u8::try_from(serial.len() + 1).map_err(|_| ReplyError::Invalid)?,
                    1,
                ]);
                wire[4..4 + serial.len()].copy_from_slice(serial);
            }
            Some(0x83) => wire[1..5].copy_from_slice(&[0x83, 2, 0, 0]),
            Some(command) => {
                // Preserve the kernel -EPROTO fit without violating the captured
                // descriptor's 63-byte feature body. The impossible message
                // length is intentionally malformed compat error framing, not
                // a physical-device reply or manufactured device data.
                wire[1] = command;
                wire[2] = u8::MAX;
            }
            None => return Err(ReplyError::Unsupported),
        }
        Ok(Report::from_wire(ReportType::Feature, true, &wire).expect("compiled feature reply"))
    }
    fn accept_output(&mut self, report: &Report) -> Result<SteamController2026Haptic, ReplyError> {
        let haptic = SteamController2026Haptic::decode(report)?;
        let index = usize::from(report.id().expect("decoded ID") - 0x80);
        if self.outputs.haptics[index].as_ref() != Some(&haptic) {
            self.outputs.haptics[index] = Some(haptic.clone());
            self.outputs.revision = self.outputs.revision.wrapping_add(1);
        }
        Ok(haptic)
    }
}
impl Protocol for SteamProtocol {
    type State = State;
    type Output = SteamController2026Haptic;
    fn neutral(&self) -> State {
        State::default()
    }
    fn validate(&self, _: &State) -> Result<(), Error> {
        if self.invalid_framing {
            Err(Error::Framing)
        } else {
            Ok(())
        }
    }
    fn input(&mut self, state: &State, _: u64) -> Result<Vec<Report>, Error> {
        self.validate(state)?;
        if self.stopped {
            return Ok(vec![]);
        }
        // steam-c6-disagreement-47-kernel/sdl: only 0x42 in the normal stream.
        self.refresh = None;
        let report = wired_input(state, self.sequence, Format::Primary42)?;
        self.sequence = self.sequence.wrapping_add(1);
        Ok(vec![report])
    }
    fn deadline(&self) -> Option<u64> {
        // steam-c6-sdl-rumble-resend-40ms is HOST pacing, not a device timer.
        // No evidence for a device rumble timeout or native report cadence.
        self.refresh
    }
    fn request(
        &mut self,
        kind: &RequestKind,
        _: u64,
    ) -> (Reply, Option<SteamController2026Haptic>) {
        match kind {
            RequestKind::Get {
                kind: ReportType::Feature,
                id: Some(1),
            } => (Reply::Get(self.get_feature()), None),
            RequestKind::Get { .. } => (Reply::Get(Err(ReplyError::Unsupported)), None),
            RequestKind::Set(report) if report.kind == ReportType::Output => {
                match self.accept_output(report) {
                    Ok(output) => (Reply::Set(Ok(())), Some(output)),
                    Err(error) => (Reply::Set(Err(error)), None),
                }
            }
            RequestKind::Set(report) => (Reply::Set(self.feature(report)), None),
        }
    }
    fn output(
        &mut self,
        report: Report,
        _: u64,
    ) -> Result<Option<SteamController2026Haptic>, Error> {
        // steam-c6-no-haptic-ack: retain outputs, never queue an input 0x44 ACK.
        self.accept_output(&report)
            .map(Some)
            .map_err(|_| Error::InvalidRequest)
    }
    fn lifecycle(&mut self, event: Lifecycle, now: u64) -> Option<SteamController2026Haptic> {
        match event {
            Lifecycle::Start {
                numbered_input,
                numbered_output,
                numbered_feature,
            } => {
                self.invalid_framing = !(numbered_input && numbered_output && numbered_feature);
                self.stopped = false;
                self.refresh = Some(now);
            }
            Lifecycle::Stop => {
                self.stopped = true;
                self.refresh = None;
                self.query = None;
            }
            Lifecycle::Open | Lifecycle::Close => {}
        }
        None
    }
    fn delivered(&mut self, _: &Command, _: Delivery) {}
}

/// UHID session adapter: the protocol retains decoded haptics privately while
/// the shared controller session reports accepted host output as raw events.
#[derive(Debug, Clone, Default)]
pub(crate) struct SteamHidProtocol {
    protocol: SteamProtocol,
    outputs: SteamController2026Outputs,
}
impl SteamHidProtocol {
    pub(crate) fn with_serial(serial: String) -> Self {
        Self::with_options(serial, false)
    }

    pub(crate) fn with_options(serial: String, steam_client_compat: bool) -> Self {
        Self {
            protocol: SteamProtocol {
                serial,
                steam_client_compat,
                ..SteamProtocol::default()
            },
            ..Self::default()
        }
    }
}
impl Protocol for SteamHidProtocol {
    type State = State;
    type Output = RawReverseEvent;

    fn neutral(&self) -> Self::State {
        self.protocol.neutral()
    }

    fn validate(&self, state: &Self::State) -> Result<(), Error> {
        self.protocol.validate(state)
    }

    fn input(&mut self, state: &Self::State, now: u64) -> Result<Vec<Report>, Error> {
        self.protocol.input(state, now)
    }

    fn deadline(&self) -> Option<u64> {
        self.protocol.deadline()
    }

    fn request(&mut self, kind: &RequestKind, now: u64) -> (Reply, Option<Self::Output>) {
        let (reply, output) = self.protocol.request(kind, now);
        let observed = output.and_then(|haptic| match kind {
            RequestKind::Set(report) => {
                self.outputs.retain(report.id(), haptic);
                Some(observe(report))
            }
            RequestKind::Get { .. } => None,
        });
        (reply, observed)
    }

    fn output(&mut self, report: Report, now: u64) -> Result<Option<Self::Output>, Error> {
        let observed = observe(&report);
        let report_id = report.id();
        self.protocol.output(report, now).map(|output| {
            output.map(|haptic| {
                self.outputs.retain(report_id, haptic);
                observed
            })
        })
    }

    fn lifecycle(&mut self, event: Lifecycle, now: u64) -> Option<Self::Output> {
        self.protocol.lifecycle(event, now);
        Some(RawReverseEvent::HidLifecycle(event))
    }

    fn delivered(&mut self, command: &Command, delivery: Delivery) {
        self.protocol.delivered(command, delivery);
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SteamController2026Outputs {
    pub revision: u64,
    pub haptics: [Option<SteamController2026Haptic>; 6],
}
impl SteamController2026Outputs {
    fn retain(&mut self, report_id: Option<u8>, haptic: SteamController2026Haptic) {
        let Some(index) = report_id
            .filter(|id| (0x80..=0x85).contains(id))
            .map(|id| usize::from(id - 0x80))
        else {
            return;
        };
        if self.haptics[index].as_ref() != Some(&haptic) {
            self.haptics[index] = Some(haptic);
            self.revision = self.revision.wrapping_add(1);
        }
    }
}
impl crate::retained::OutputState for SteamController2026Outputs {
    fn apply(&mut self, report: &Report) -> Result<(), ReplyError> {
        self.retain(report.id(), SteamController2026Haptic::decode(report)?);
        Ok(())
    }

    fn from_native(_: crate::retained::NativePlayback) -> Self {
        Self::default()
    }
}
impl crate::retained::RetainedProtocol for SteamHidProtocol {
    type Outputs = SteamController2026Outputs;

    fn retained(&self) -> &Self::Outputs {
        &self.outputs
    }
}

fn observe(report: &Report) -> RawReverseEvent {
    RawReverseEvent::HidOutput {
        report_id: report.id(),
        bytes: report.payload().to_vec(),
    }
}

#[cfg(test)]
mod tests;
