//! Wired native/feature/haptic compatibility plus opt-in desktop reports.
//! Desktop mappings and bounded refresh policy: steam-d9-lizard-control-mapping,
//! steam-d9-lizard-sdl-transition and steam-d9-lizard-relative-scaling.
use super::streaming::{
    SteamController2026StreamOutputs, SteamController2026StreamPacket, StreamState,
};
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
    /// Opt-in, captured haptic PCM request; raw callbacks preserve every write.
    Stream(Box<SteamController2026StreamPacket>),
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
        // steam-d9-midi-lfo-framing: the physical Singer client sends output
        // 0x83 in a 64-byte zero-padded buffer. Interpret only its declared
        // nine-byte body; raw observations still preserve the complete write.
        // Nonzero extensions are unreviewed and remain invalid.
        let b = if report.id() == Some(0x83) && b.len() == 63 && b[9..].iter().all(|v| *v == 0) {
            &b[..9]
        } else {
            b
        };
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
            (Some(0x86..=0x88), _) => {
                Self::Stream(Box::new(SteamController2026StreamPacket::decode(report)?))
            }
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
    rgbw: Option<[u8; 4]>,
    // Last accepted setting intent; feature 0x89 reads key 37 only.
    // Other entries do not imply general readback or persistent settings.
    compat_settings: std::collections::BTreeMap<u8, u16>,
}
// steam-d9-named-setting-framing / steam-d9-wired-entry-stability-consumer.
// Exact protocol constants only; these never address the host filesystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NamedRead {
    Transport,
    AbsentRecord,
}
impl NamedRead {
    fn parse(body: &[u8]) -> Option<Self> {
        let length = usize::from(*body.get(1)?);
        match body.get(2..2 + length)? {
            b"user/wireless_transport\0" => Some(Self::Transport),
            b"esb/bond\0" | b"esb/bond_2\0" => Some(Self::AbsentRecord),
            _ => None,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Query {
    Serial,
    Attributes,
    Named(NamedRead),
    LedColor,
    LedOverride,
    Unsupported(u8),
}
#[derive(Debug, Clone, Default)]
struct SteamProtocol {
    desktop: super::lizard::Desktop,
    serial: String,
    steam_client_compat: bool,
    stream: Box<StreamState>,
    sequence: u8,
    outputs: Outputs,
    query: Option<Query>,
    stopped: bool,
    invalid_framing: bool,
    refresh: Option<u64>,
}
impl SteamProtocol {
    fn feature(&mut self, report: &Report) -> Result<(), ReplyError> {
        // Every SET invalidates the preceding query, even an invalid report ID.
        self.query = None;
        // steam-c6-no-wired-feature2: do not serve receiver feature 2 on wired.
        if report.kind != ReportType::Feature || report.id() != Some(1) {
            return Err(ReplyError::Unsupported);
        }
        // steam-c6-feature-kernel-exchange: fixed 64-byte numbered host buffer.
        let b = report.payload();
        if b.len() != 63 {
            return Err(ReplyError::Invalid);
        }
        let mut next = self.outputs.clone();
        match b[0] {
            // hid-steam v7.3-rc1, lines 667–685: current request and documented historical length.
            0xae if matches!(b[1], 0x15 | 0x16) && b[2] == 1 => self.query = Some(Query::Serial),
            0x83 if b[1] == 0 => self.query = Some(Query::Attributes),
            // steam-d9-feature-response-framing: these exact two-byte SET
            // bodies completed successfully. No semantics or retained state
            // are inferred from that transport acknowledgement.
            0xdc if self.steam_client_compat && b[1..4] == [2, 1, 2] => {}
            0xe2 if self.steam_client_compat && b[1..4] == [2, 1, 0x20] => {}
            // Physical zero transport scalar; synthetic absent records, never
            // physical pairing material or opaque reserved tails.
            0xed if self.steam_client_compat && NamedRead::parse(b).is_some() => {
                self.query = NamedRead::parse(b).map(Query::Named);
            }
            // steam-d9-wired-rgbw-override: raw intent is retained even when
            // the override is disabled; visibility and intent are separate.
            0xc5 if self.steam_client_compat => {
                if b[1] != 4 {
                    return Err(ReplyError::Invalid);
                }
                next.rgbw = Some(b[2..6].try_into().expect("four colour bytes"));
            }
            0xe9 if self.steam_client_compat => {
                if b[1] != 0 {
                    return Err(ReplyError::Invalid);
                }
                self.query = Some(Query::LedColor);
            }
            0x89 if self.steam_client_compat && b[1..3] == [1, 37] => {
                self.query = Some(Query::LedOverride);
            }
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
                        37 if self.steam_client_compat && value <= 1 => {}
                        37 if self.steam_client_compat => return Err(ReplyError::Invalid),
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
            // Update and bootloader operations are unavailable. The query was
            // cleared before parsing: a following GET must also be Unsupported,
            // never a compatibility exchange or acceptance of update data.
            0x90 | 0x9f => return Err(ReplyError::Unsupported),
            0xae | 0x83 | 0x81 | 0x85 | 0x8e => return Err(ReplyError::Invalid),
            _ => {
                // steam-c6-feature-kernel-exchange / reply-attributes-short:
                // reject SET, and provide a deliberately malformed exchange reply
                // if a host still GETs it. Kernel detects -EPROTO, not stale data.
                // This fits the kernel error path; it is not device behaviour.
                self.query = Some(Query::Unsupported(b[0]));
                return Err(ReplyError::Unsupported);
            }
        }
        // Clear cached input history at the accepted transition, even if host
        // disable/enable requests are batched before the next input cycle.
        // Rejected setting batches leave both intent and input history intact.
        if next.mappings != self.outputs.mappings || matches!(b[0], 0x81 | 0x85) {
            self.desktop.reset();
        } else if next.lizard != self.outputs.lizard || b[0] == 0x8e {
            self.desktop.clear_mouse();
        }
        if next != self.outputs {
            next.revision = self.outputs.revision.wrapping_add(1);
            self.outputs = next;
        }
        Ok(())
    }
    fn get_feature(&self) -> Result<Report, ReplyError> {
        if self.steam_client_compat && self.query == Some(Query::Attributes) {
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
            Some(Query::Named(named)) => {
                wire[1] = 0xed;
                wire[2] = u8::from(named == NamedRead::Transport);
                wire.truncate(63);
            }
            Some(Query::LedColor) => {
                // steam-d9-led-colour-readback-quantization: physical E9
                // replies were four zero bytes at the tested raw values 25/10.
                // Higher-value readback is unverified; do not derive it from intent.
                wire[1..3].copy_from_slice(&[0xe9, 4]);
                wire.truncate(63);
            }
            Some(Query::LedOverride) => {
                let enabled = self.outputs.compat_settings.get(&37).copied().unwrap_or(0);
                wire[1..6].copy_from_slice(&[0x89, 3, 37, u8::from(enabled != 0), 0]);
                wire.truncate(63);
            }
            Some(Query::Serial) => {
                let serial = self.serial.as_bytes();
                wire[1..4].copy_from_slice(&[
                    0xae,
                    u8::try_from(serial.len() + 1).map_err(|_| ReplyError::Invalid)?,
                    1,
                ]);
                wire[4..4 + serial.len()].copy_from_slice(serial);
            }
            Some(Query::Attributes) => wire[1..5].copy_from_slice(&[0x83, 2, 0, 0]),
            Some(Query::Unsupported(command)) => {
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
    fn accept_output(
        &mut self,
        report: &Report,
        now: u64,
    ) -> Result<SteamController2026Haptic, ReplyError> {
        if matches!(report.id(), Some(0x86..=0x89)) {
            if !self.stream.outputs.enabled {
                return Err(ReplyError::Unsupported);
            }
            if self.stopped {
                return Err(ReplyError::Busy);
            }
        }
        let haptic = SteamController2026Haptic::decode(report)?;
        if let SteamController2026Haptic::Stream(packet) = &haptic {
            self.stream.accept(packet, now)?;
            return Ok(haptic);
        }
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
    fn validate(&self, state: &State) -> Result<(), Error> {
        if self.invalid_framing {
            Err(Error::Framing)
        } else {
            super::definition::validate_trigger_values(state).map_err(|_| Error::InvalidState)
        }
    }
    fn input(&mut self, state: &State, now: u64) -> Result<Vec<Report>, Error> {
        self.validate(state)?;
        if self.stopped {
            return Ok(vec![]);
        }
        // steam-c6-disagreement-47-kernel/sdl: only 0x42 in the normal stream.
        self.refresh = None;
        let report = wired_input(state, self.sequence, Format::Primary42)?;
        self.sequence = self.sequence.wrapping_add(1);
        let mut reports = vec![report];
        reports.extend(self.desktop.input(
            state,
            self.outputs.mappings.unwrap_or(true),
            self.outputs.lizard.unwrap_or(true),
        ));
        if self.desktop.repeating() {
            self.refresh = Some(now.saturating_add(4_000));
        }
        reports.extend(self.stream.poll(now).map_err(|_| Error::QueueFull)?);
        Ok(reports)
    }
    fn deadline(&self) -> Option<u64> {
        // steam-c6-sdl-rumble-resend-40ms is HOST pacing, not a device timer.
        // No evidence for a device rumble timeout or native report cadence.
        self.refresh.into_iter().chain(self.stream.deadline()).min()
    }
    fn request(
        &mut self,
        kind: &RequestKind,
        now: u64,
    ) -> (Reply, Option<SteamController2026Haptic>) {
        match kind {
            RequestKind::Get {
                kind: ReportType::Feature,
                id: Some(1),
            } => (Reply::Get(self.get_feature()), None),
            RequestKind::Get { .. } => (Reply::Get(Err(ReplyError::Unsupported)), None),
            RequestKind::Set(report) if report.kind == ReportType::Output => {
                match self.accept_output(report, now) {
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
        now: u64,
    ) -> Result<Option<SteamController2026Haptic>, Error> {
        // Legacy haptics have no input ACK. Opt-in PCM queues the captured
        // stream statuses; these are not one acknowledgement per data write.
        self.accept_output(&report, now).map(Some).map_err(|error| {
            if error == ReplyError::Busy {
                Error::QueueFull
            } else {
                Error::InvalidRequest
            }
        })
    }
    fn lifecycle(&mut self, event: Lifecycle, now: u64) -> Option<SteamController2026Haptic> {
        match event {
            Lifecycle::Start {
                numbered_input,
                numbered_output,
                numbered_feature,
            } => {
                self.desktop.reset();
                self.stream.reset(now);
                self.invalid_framing = !(numbered_input && numbered_output && numbered_feature);
                self.stopped = false;
                self.refresh = Some(now);
            }
            Lifecycle::Stop => {
                self.desktop.reset();
                self.stream.reset(now);
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
        Self::with_streaming_options(serial, steam_client_compat, false)
    }

    pub(crate) fn with_streaming_options(
        serial: String,
        steam_client_compat: bool,
        haptic_streaming: bool,
    ) -> Self {
        Self::with_desktop_options(
            serial,
            steam_client_compat,
            haptic_streaming,
            super::SteamController2026CreationOptions::default().lizard_mode,
        )
    }

    pub(crate) fn with_desktop_options(
        serial: String,
        steam_client_compat: bool,
        haptic_streaming: bool,
        lizard_mode: bool,
    ) -> Self {
        let mut protocol = Self {
            protocol: SteamProtocol {
                desktop: super::lizard::Desktop::new(lizard_mode),
                serial,
                steam_client_compat,
                stream: Box::new(StreamState::new(haptic_streaming)),
                ..SteamProtocol::default()
            },
            ..Self::default()
        };
        protocol.sync_outputs();
        protocol
    }

    fn sync_outputs(&mut self) {
        let brightness = self.protocol.outputs.compat_settings.get(&45).copied();
        let override_enabled = self
            .protocol
            .outputs
            .compat_settings
            .get(&37)
            .map(|value| *value != 0);
        let rgbw = self.protocol.outputs.rgbw;
        if self.outputs.streaming != self.protocol.stream.outputs
            || self.outputs.led_brightness != brightness
            || self.outputs.led_override != override_enabled
            || self.outputs.led_rgbw != rgbw
        {
            self.outputs.streaming = self.protocol.stream.outputs.clone();
            self.outputs.led_brightness = brightness;
            self.outputs.led_override = override_enabled;
            self.outputs.led_rgbw = rgbw;
            self.outputs.revision = self.outputs.revision.wrapping_add(1);
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
        let reports = self.protocol.input(state, now);
        self.sync_outputs();
        reports
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
        self.sync_outputs();
        (reply, observed)
    }

    fn output(&mut self, report: Report, now: u64) -> Result<Option<Self::Output>, Error> {
        let observed = observe(&report);
        let report_id = report.id();
        let result = self.protocol.output(report, now).map(|output| {
            output.map(|haptic| {
                self.outputs.retain(report_id, haptic);
                observed
            })
        });
        self.sync_outputs();
        result
    }

    fn lifecycle(&mut self, event: Lifecycle, now: u64) -> Option<Self::Output> {
        self.protocol.lifecycle(event, now);
        self.sync_outputs();
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
    pub streaming: SteamController2026StreamOutputs,
    /// Last accepted raw LED-brightness intent (setting 45), not RGB or readback.
    ///
    /// `steam-d9-wired-led-brightness` observes percent-like wired values. No
    /// calibrated transfer function or persistence is asserted. `None` means
    /// unobserved, or unknown after restore-default intent. Requires the Steam
    /// compatibility profile; reduced uinput has no HID feature channel.
    pub led_brightness: Option<u16>,
    /// Last accepted setting-37 override intent; `None` is unknown.
    ///
    /// `steam-d9-wired-rgbw-override`: enabling this exposes the four RGBW
    /// channels instead of normal status colour. Restore-default clears this
    /// retained knowledge; reduced uinput has no corresponding feature channel.
    pub led_override: Option<bool>,
    /// Last accepted C5 bytes, ordered red/green/blue/white, independently of override.
    ///
    /// `steam-d9-wired-rgbw-override` observes raw per-channel values 25 and 0.
    /// This is accepted raw intent, not calibrated intensity. E9 returned zeros
    /// at the tested values; higher-value readback remains unverified
    /// (`steam-d9-led-colour-readback-quantization`). No uinput equivalent exists.
    pub led_rgbw: Option<[u8; 4]>,
}
impl SteamController2026Outputs {
    fn retain(&mut self, report_id: Option<u8>, haptic: SteamController2026Haptic) {
        if let SteamController2026Haptic::Stream(packet) = haptic {
            self.streaming.last_packet = Some(*packet);
            self.revision = self.revision.wrapping_add(1);
            return;
        }
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
        let haptic = SteamController2026Haptic::decode(report)?;
        if matches!(haptic, SteamController2026Haptic::Stream(_)) {
            return Err(ReplyError::Unsupported); // Native evdev has no PCM stream channel.
        }
        self.retain(report.id(), haptic);
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
