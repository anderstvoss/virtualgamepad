//! Demo-owned bounded PCM work; only the root application API is used.
use super::audio_topology::{AudioTopology, ControllerFamily, JackConnector, JackDevice};
use eframe::egui;
use std::collections::HashMap;
use virtualgamepad::{
    AudioAccess, AudioDiagnostics, AudioEndpoint, AudioError, AudioExposure, AudioOptions,
    ControllerAudio, ControllerError, CreationOptions, PcmFormat, RealizationId, SampleDirection,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum HostBackend {
    #[default]
    PipeWire,
    Alsa,
}

impl HostBackend {
    const ALL: [Self; 2] = [Self::PipeWire, Self::Alsa];
    const fn label(self) -> &'static str {
        match self {
            Self::PipeWire => "PipeWire",
            Self::Alsa => "ALSA",
        }
    }

    fn compiled(self) -> bool {
        match self {
            Self::PipeWire => cfg!(all(target_os = "linux", feature = "audio-pipewire")),
            Self::Alsa => cfg!(all(target_os = "linux", feature = "audio-alsa")),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct HostDevice {
    pub id: String,
    pub label: String,
}

#[derive(Default)]
pub(super) struct AudioRouter {
    config: Option<(RoutingState, bool, bool)>,
    outputs: HashMap<String, super::host_audio::OutputPort>,
    inputs: HashMap<String, super::host_audio::InputPort>,
    last_error: Option<String>,
}

impl AudioRouter {
    fn configure(
        &mut self,
        routing: &RoutingState,
        sample_playback: bool,
        sample_capture: bool,
    ) -> Option<String> {
        if self.config.as_ref() == Some(&(routing.clone(), sample_playback, sample_capture)) {
            return self.last_error.clone();
        }
        self.outputs.clear();
        self.inputs.clear();
        self.config = Some((routing.clone(), sample_playback, sample_capture));
        if !routing.backend.compiled() {
            self.last_error = Some(format!(
                "{} host routing is not built",
                routing.backend.label()
            ));
            return self.last_error.clone();
        }
        let (allow_playback, allow_capture, mut errors) =
            sample_route_ownership(routing, sample_playback, sample_capture);
        for output in routing
            .outputs
            .iter()
            .filter(|route| route.enabled && allow_playback)
        {
            match super::host_audio::OutputPort::open(
                routing.backend,
                &output.device_id,
                output.channels,
            ) {
                Ok(port) => {
                    self.outputs.insert(output.id.clone(), port);
                }
                Err(error) => errors.push(format!("{}: {error}", output.label)),
            }
        }
        for input in routing
            .inputs
            .iter()
            .filter(|route| route.enabled && allow_capture)
        {
            let channels = input.source_channels.clamp(1, 32);
            match super::host_audio::InputPort::open(routing.backend, &input.device_id, channels) {
                Ok(port) => {
                    self.inputs.insert(input.id.clone(), port);
                }
                Err(error) => errors.push(format!("{}: {error}", input.label)),
            }
        }
        self.last_error = (!errors.is_empty()).then(|| errors.join("; "));
        self.last_error.clone()
    }

    fn output(&self, id: &str, samples: &[i16]) -> bool {
        self.outputs.get(id).is_some_and(|port| port.write(samples))
    }

    fn input(&self, id: &str) -> Option<Vec<i16>> {
        let port = self.inputs.get(id)?;
        let mut newest = port.read();
        while let Some(next) = port.read() {
            newest = Some(next);
        }
        newest
    }

    fn error(&self) -> Option<String> {
        let mut errors: Vec<_> = self
            .outputs
            .iter()
            .filter_map(|(id, port)| port.error().map(|error| format!("{id}: {error}")))
            .chain(
                self.inputs
                    .iter()
                    .filter_map(|(id, port)| port.error().map(|error| format!("{id}: {error}"))),
            )
            .collect();
        let dropped: u64 = self
            .outputs
            .values()
            .map(super::host_audio::OutputPort::dropped_packets)
            .sum();
        if dropped > 0 {
            errors.push(format!("host playback dropped {dropped} audio blocks"));
        }
        (!errors.is_empty()).then(|| errors.join("; "))
    }

    fn reset(&mut self) {
        self.config = None;
    }
}

fn sample_route_ownership(
    routing: &RoutingState,
    sample_playback: bool,
    sample_capture: bool,
) -> (bool, bool, Vec<String>) {
    let playback = sample_playback;
    let capture = sample_capture;
    let mut errors = Vec::new();
    if !playback && routing.outputs.iter().any(|route| route.enabled) {
        errors.push(
            "controller owns playback as a native client; sample-based host routing is unavailable"
                .into(),
        );
    }
    if !capture && routing.inputs.iter().any(|route| route.enabled) {
        errors.push(
            "controller owns microphone capture as a native client; sample-based host routing is unavailable"
                .into(),
        );
    }
    (playback, capture, errors)
}

fn host_devices(backend: HostBackend) -> [HostDevice; 1] {
    [HostDevice {
        id: "default".into(),
        label: match backend {
            HostBackend::PipeWire => "System default output/input".into(),
            HostBackend::Alsa => "ALSA default device".into(),
        },
    }]
}

fn discovered_devices(
    backend: HostBackend,
    direction: super::host_audio::Direction,
) -> Vec<HostDevice> {
    if !backend.compiled() {
        return host_devices(backend).to_vec();
    }
    super::host_audio::enumerate(backend, direction)
        .unwrap_or_else(|_| host_devices(backend).to_vec())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct InputRoute {
    pub id: String,
    pub label: String,
    pub enabled: bool,
    pub device_id: String,
    pub source_channels: usize,
    pub target_channels: Vec<Option<usize>>,
    pub peak: Vec<u16>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct OutputRoute {
    pub id: String,
    pub label: String,
    pub channels: usize,
    pub enabled: bool,
    pub device_id: String,
    pub source_channels: Vec<Option<usize>>,
    pub peak: Vec<u16>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RoutingState {
    pub backend: HostBackend,
    pub jack_connector: Option<JackConnector>,
    pub outputs: Vec<OutputRoute>,
    pub inputs: Vec<InputRoute>,
    pub jack_connected: bool,
    pub jack_device: JackDevice,
}

impl RoutingState {
    fn for_topology(topology: AudioTopology, backend: HostBackend) -> Self {
        let mut state = Self {
            backend,
            jack_connector: topology.jack,
            outputs: Vec::new(),
            inputs: Vec::new(),
            jack_connected: false,
            jack_device: JackDevice::Headset,
        };
        if topology.onboard_speaker_channels > 0 {
            state.outputs.push(OutputRoute {
                id: "onboard-speaker".into(),
                label: "Built-in speaker".into(),
                channels: topology.onboard_speaker_channels,
                enabled: true,
                device_id: "default".into(),
                source_channels: (0..topology.onboard_speaker_channels).map(Some).collect(),
                peak: vec![0; topology.onboard_speaker_channels],
            });
        }
        if topology.onboard_microphone_channels > 0 {
            state.inputs.push(InputRoute {
                id: "onboard-microphone".into(),
                label: "Built-in microphone".into(),
                enabled: false,
                device_id: "default".into(),
                source_channels: topology.onboard_microphone_channels,
                target_channels: (0..topology.onboard_microphone_channels)
                    .map(Some)
                    .collect(),
                peak: vec![0; topology.onboard_microphone_channels],
            });
        }
        state.set_jack_connected(false, topology);
        state
    }

    fn set_jack_connected(&mut self, connected: bool, topology: AudioTopology) {
        self.jack_connected = connected;
        self.outputs.retain(|output| output.id != "jack-output");
        self.inputs.retain(|input| input.id != "jack-microphone");
        if let Some(connector) = topology
            .jack
            .filter(|_| connected && self.jack_device.output_channels() > 0)
        {
            let channels = self.jack_device.output_channels();
            self.outputs.push(OutputRoute {
                id: "jack-output".into(),
                label: format!("{} output", self.jack_device.label_for_connector(connector)),
                channels,
                enabled: true,
                device_id: "default".into(),
                source_channels: (0..channels).map(Some).collect(),
                peak: vec![0; channels],
            });
        }
        if let Some(connector) = topology
            .jack
            .filter(|_| connected && self.jack_device.input_channels() > 0)
        {
            let channels = self.jack_device.input_channels();
            self.inputs.push(InputRoute {
                id: "jack-microphone".into(),
                label: format!(
                    "{} microphone",
                    self.jack_device.label_for_connector(connector)
                ),
                enabled: false,
                device_id: "default".into(),
                source_channels: channels,
                target_channels: vec![Some(0)],
                peak: vec![0; channels],
            });
        }
        if topology.jack.is_none() {
            self.jack_connected = false;
        }
    }

    fn set_jack_device(&mut self, device: JackDevice, topology: AudioTopology) {
        self.jack_device = device;
        self.set_jack_connected(self.jack_connected, topology);
    }
}

#[derive(Clone, Copy)]
pub(super) struct CreationAudio {
    pub enabled: bool,
    pub playback: AudioAccess,
    pub microphone: AudioAccess,
    pub host_backend: HostBackend,
}
impl Default for CreationAudio {
    fn default() -> Self {
        Self {
            enabled: default_creation_target() == RealizationId::LINUX_UHID_USB,
            playback: AudioAccess::Samples,
            microphone: AudioAccess::Samples,
            host_backend: default_host_backend(),
        }
    }
}

pub(super) fn default_creation_target() -> RealizationId {
    if cfg!(all(target_os = "linux", feature = "audio-pipewire")) {
        RealizationId::LINUX_UHID_USB
    } else {
        RealizationId::LINUX_UINPUT
    }
}

fn default_host_backend() -> HostBackend {
    if HostBackend::PipeWire.compiled() {
        HostBackend::PipeWire
    } else if HostBackend::Alsa.compiled() {
        HostBackend::Alsa
    } else {
        HostBackend::PipeWire
    }
}

pub(super) fn default_audio_enabled(target: RealizationId, has_audio: bool) -> bool {
    has_audio && backend_available(target)
}
impl CreationAudio {
    pub fn options(
        self,
        target: RealizationId,
        has_audio: bool,
    ) -> Result<CreationOptions, ControllerError> {
        if self.enabled && (!has_audio || !backend_available(target)) {
            return Err(ControllerError::Unsupported { reason: "Emulated audio needs an audio-capable family, a compiled audio feature and UHID or USB/IP. Select another creation choice or disable audio.".into() });
        }
        if self.enabled && !self.host_backend.compiled() {
            return Err(ControllerError::Unsupported {
                reason: format!(
                    "{} host routing is not compiled into this demo.",
                    self.host_backend.label()
                ),
            });
        }
        if target == RealizationId::LINUX_USBIP_USB_AUDIO && !self.enabled {
            return Err(ControllerError::Unsupported {
                reason: "USB/IP HID/UAC2 is composite and requires emulated audio enabled.".into(),
            });
        }
        if self.enabled
            && target == RealizationId::LINUX_USBIP_USB_AUDIO
            && (self.playback == AudioAccess::NativeClient
                || self.microphone == AudioAccess::NativeClient)
            && !cfg!(feature = "audio-pipewire")
        {
            return Err(ControllerError::Unsupported {
                reason: "USB/IP native clients also need the audio-pipewire demo feature.".into(),
            });
        }
        Ok(CreationOptions::new(target).with_audio(
            AudioOptions::new(if self.enabled {
                AudioExposure::Emulated
            } else {
                AudioExposure::Disabled
            })
            .with_playback_access(self.playback)
            .with_microphone_access(self.microphone),
        ))
    }
}
fn backend_available(target: RealizationId) -> bool {
    (target == RealizationId::LINUX_UHID_USB
        && cfg!(all(target_os = "linux", feature = "audio-pipewire")))
        || (target == RealizationId::LINUX_USBIP_USB_AUDIO
            && cfg!(all(target_os = "linux", feature = "audio-usbip")))
}

fn draw_labeled_combo_box(
    ui: &mut egui::Ui,
    label: &str,
    id: impl std::hash::Hash,
    selected_text: &str,
    enabled: bool,
    add_options: impl FnOnce(&mut egui::Ui),
) {
    ui.label(label);
    ui.add_enabled_ui(enabled, |ui| {
        egui::ComboBox::from_id_salt(id)
            .selected_text(selected_text)
            .width((ui.available_width() / 2.0 - 4.0).max(80.0))
            .show_ui(ui, add_options);
    });
    ui.end_row();
}

pub(super) fn draw_creation(
    ui: &mut egui::Ui,
    config: &mut CreationAudio,
    target: RealizationId,
    has_audio: bool,
) -> egui::Rect {
    let validation = config.options(target, has_audio);
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        egui::Grid::new("audio_creation_grid")
            .num_columns(2)
            .max_col_width(80.0)
            .spacing([6.0, 4.0])
            .show(ui, |ui| {
                ui.label("Audio");
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut config.enabled, "Emulated audio");
                    let info = ui.add_sized([18.0, 18.0], egui::Button::new("!"));
                    if info.hovered() {
                        egui::Tooltip::for_widget(&info).at_pointer().show(|ui| {
                            ui.strong("Audio creation details");
                            ui.label("Audio endpoints and the selected host backend are created with the controller. Recreate it to change those choices.");
                            ui.label("Samples ownership routes speaker output to the system default device. Microphone piping starts disabled.");
                            ui.label("PipeWire routing needs wpctl/pw-cat. ALSA routing needs aplay/arecord from alsa-utils.");
                            if target == RealizationId::LINUX_USBIP_USB_AUDIO {
                                ui.label("USB/IP is opt-in WIP; installed security and recovery acceptance remains open.");
                            }
                            if let Err(error) = &validation {
                                ui.colored_label(egui::Color32::YELLOW, error.to_string());
                            }
                        });
                    }
                });
                ui.end_row();

                let status = if !has_audio {
                    Some("Unsupported for this family")
                } else if !backend_available(target) {
                    Some("Backend unavailable")
                } else if target == RealizationId::LINUX_USBIP_USB_AUDIO && !config.enabled {
                    Some("USB/IP requires audio")
                } else {
                    None
                };
                if let Some(status) = status {
                    ui.label("Status");
                    ui.colored_label(egui::Color32::YELLOW, status);
                    ui.end_row();
                }

                if config.enabled && has_audio {
                    let available = backend_available(target);
                    draw_labeled_combo_box(
                        ui,
                        "Host audio",
                        "host_audio_backend",
                        config.host_backend.label(),
                        available,
                        |ui| {
                            for backend in HostBackend::ALL {
                                let label = if backend.compiled() {
                                    backend.label().to_owned()
                                } else {
                                    format!("{} (not built)", backend.label())
                                };
                                ui.add_enabled_ui(backend.compiled(), |ui| {
                                    ui.selectable_value(
                                        &mut config.host_backend,
                                        backend,
                                        label,
                                    );
                                });
                            }
                        },
                    );
                    for (label, id, access) in [
                        ("Playback", "playback_ownership", &mut config.playback),
                        (
                            "Microphone",
                            "microphone_ownership",
                            &mut config.microphone,
                        ),
                    ] {
                        draw_labeled_combo_box(
                            ui,
                            label,
                            id,
                            match *access {
                                AudioAccess::Samples => "Samples",
                                AudioAccess::NativeClient => "Native client",
                                _ => "Unavailable",
                            },
                            available,
                            |ui| {
                                ui.selectable_value(access, AudioAccess::Samples, "Samples");
                                ui.selectable_value(
                                    access,
                                    AudioAccess::NativeClient,
                                    "Native client",
                                );
                            },
                        );
                    }
                }
            });
    })
    .response
    .rect
}

#[derive(Clone, Debug)]
pub(super) enum Action {
    FlushPlayback,
    FlushMicrophone,
    Tone(bool),
    Routing(RoutingState),
    RetryRouting,
}
#[derive(Clone, Debug)]
pub(super) struct Activity {
    pub playback_frames: u64,
    pub microphone_frames: u64,
    pub discontinuities: u64,
    pub first_playback_frame: Option<u64>,
    pub peak: u16,
    pub tone: bool,
    pub routing: RoutingState,
    pub playback_devices: Vec<HostDevice>,
    pub capture_devices: Vec<HostDevice>,
    pub routing_error: Option<String>,
    initialized: bool,
    pending: Vec<i16>,
    accepted_samples: usize,
}
impl Default for Activity {
    fn default() -> Self {
        Self::empty(default_host_backend())
    }
}
impl Activity {
    pub(super) fn initialize_for_family(&mut self, family: ControllerFamily, backend: HostBackend) {
        if !self.initialized {
            *self = Self::for_family(family, backend);
        }
    }

    fn empty(backend: HostBackend) -> Self {
        Self {
            playback_frames: 0,
            microphone_frames: 0,
            discontinuities: 0,
            first_playback_frame: None,
            peak: 0,
            tone: false,
            routing: RoutingState::for_topology(
                AudioTopology::for_family(ControllerFamily::SwitchPro),
                backend,
            ),
            playback_devices: Vec::new(),
            capture_devices: Vec::new(),
            routing_error: None,
            initialized: false,
            pending: Vec::new(),
            accepted_samples: 0,
        }
    }

    pub fn for_family(family: ControllerFamily, backend: HostBackend) -> Self {
        Self {
            playback_frames: 0,
            microphone_frames: 0,
            discontinuities: 0,
            first_playback_frame: None,
            peak: 0,
            tone: false,
            routing: RoutingState::for_topology(AudioTopology::for_family(family), backend),
            playback_devices: discovered_devices(backend, super::host_audio::Direction::Playback),
            capture_devices: discovered_devices(backend, super::host_audio::Direction::Capture),
            routing_error: None,
            initialized: true,
            pending: Vec::new(),
            accepted_samples: 0,
        }
    }
}
// Per cycle: one read and one write, each at most 512 complete frames.
const FRAMES: usize = 512;
struct ReadObservation {
    first_frame: u64,
    frames: usize,
    discontinuity: bool,
}
trait SampleIo {
    fn read(&mut self, samples: &mut [i16]) -> Result<ReadObservation, AudioError>;
    fn write(&mut self, samples: &[i16]) -> Result<usize, AudioError>;
    fn flush(&mut self, direction: SampleDirection) -> Result<(), AudioError>;
}
impl SampleIo for ControllerAudio {
    fn read(&mut self, samples: &mut [i16]) -> Result<ReadObservation, AudioError> {
        self.read_playback(samples).map(|read| ReadObservation {
            first_frame: read.first_frame,
            frames: read.frames,
            discontinuity: read.discontinuity,
        })
    }
    fn write(&mut self, samples: &[i16]) -> Result<usize, AudioError> {
        self.write_microphone(samples)
    }
    fn flush(&mut self, direction: SampleDirection) -> Result<(), AudioError> {
        match direction {
            SampleDirection::HostToController => self.flush_playback(),
            SampleDirection::ControllerToHost => self.flush_microphone(),
            _ => Err(AudioError::Unsupported),
        }
    }
}
impl Activity {
    fn apply<I: SampleIo>(&mut self, io: &mut I, action: Action) -> Result<(), AudioError> {
        match action {
            Action::FlushPlayback => io.flush(SampleDirection::HostToController),
            Action::FlushMicrophone => {
                io.flush(SampleDirection::ControllerToHost)?;
                self.pending.clear();
                self.accepted_samples = 0;
                Ok(())
            }
            Action::Tone(enabled) => {
                // Stop discards unaccepted samples and queued microphone audio.
                if !enabled {
                    io.flush(SampleDirection::ControllerToHost)?;
                }
                self.tone = enabled;
                self.pending.clear();
                self.accepted_samples = 0;
                Ok(())
            }
            Action::Routing(routing) => {
                self.routing = routing;
                Ok(())
            }
            Action::RetryRouting => Ok(()),
        }
    }
    #[allow(clippy::too_many_lines)]
    fn tick<I: SampleIo>(
        &mut self,
        io: &mut I,
        router: &AudioRouter,
        playback: Option<&PcmFormat>,
        microphone: Option<&PcmFormat>,
    ) -> Result<(), AudioError> {
        if let Some(format) = playback {
            let channels = format.channels().len();
            let mut buffer = vec![0; FRAMES * channels];
            let read = io.read(&mut buffer)?;
            if read.frames != 0 {
                self.first_playback_frame = Some(read.first_frame);
            }
            self.playback_frames = self.playback_frames.saturating_add(read.frames as u64);
            self.discontinuities = self
                .discontinuities
                .saturating_add(u64::from(read.discontinuity));
            self.peak = buffer[..read.frames * channels]
                .iter()
                .map(|n| n.unsigned_abs())
                .max()
                .unwrap_or(0);
            let source = &buffer[..read.frames * channels];
            for route in &mut self.routing.outputs {
                if !route.enabled {
                    route.peak.fill(0);
                    continue;
                }
                let mut samples = vec![0; read.frames * route.channels];
                let mappings: Vec<_> = route
                    .source_channels
                    .iter()
                    .enumerate()
                    .map(
                        |(destination, source)| super::audio_topology::ChannelRoute {
                            source: *source,
                            destination,
                        },
                    )
                    .collect();
                super::audio_topology::apply_channel_routes(
                    source,
                    channels,
                    &mut samples,
                    route.channels,
                    &mappings,
                );
                route.peak = (0..route.channels)
                    .map(|channel| {
                        samples
                            .iter()
                            .skip(channel)
                            .step_by(route.channels)
                            .map(|sample| sample.unsigned_abs())
                            .max()
                            .unwrap_or(0)
                    })
                    .collect();
                if !samples.is_empty() {
                    let _ = router.output(&route.id, &samples);
                }
            }
        }
        if let Some(format) = microphone {
            let channels = format.channels().len();
            let mut microphone_samples = vec![0_i16; FRAMES * channels];
            let mut has_input = false;
            for route in &mut self.routing.inputs {
                if !route.enabled {
                    route.peak.fill(0);
                    continue;
                }
                let Some(source) = router.input(&route.id) else {
                    continue;
                };
                let source_channels = route.source_channels.clamp(1, 32);
                let mappings: Vec<_> = route
                    .target_channels
                    .iter()
                    .copied()
                    .enumerate()
                    .map(
                        |(source, destination)| super::audio_topology::ChannelRoute {
                            source: destination.map(|_| source),
                            destination: destination.unwrap_or(0),
                        },
                    )
                    .collect();
                route.peak = super::audio_topology::mix_channel_routes(
                    &source,
                    source_channels,
                    &mut microphone_samples,
                    channels,
                    &mappings,
                );
                has_input |= route
                    .target_channels
                    .iter()
                    .flatten()
                    .any(|destination| *destination < channels);
            }
            if self.tone {
                for frame in 0..FRAMES {
                    let position = self.microphone_frames.saturating_add(frame as u64);
                    let sample =
                        if (position.saturating_mul(880) / u64::from(format.sample_rate_hz())) % 2
                            == 0
                        {
                            1024
                        } else {
                            -1024
                        };
                    for channel in 0..channels {
                        let destination = &mut microphone_samples[frame * channels + channel];
                        *destination = destination.saturating_add(sample);
                    }
                }
                has_input = true;
            }
            if self.pending.is_empty() && has_input {
                self.pending = microphone_samples;
            }
            if !self.pending.is_empty() {
                let frames = io.write(&self.pending[self.accepted_samples..])?;
                self.microphone_frames = self.microphone_frames.saturating_add(frames as u64);
                self.accepted_samples += frames * channels;
                if self.accepted_samples == self.pending.len() {
                    self.pending.clear();
                    self.accepted_samples = 0;
                }
            }
        }
        Ok(())
    }

    #[cfg(test)]
    fn tick_without_routes<I: SampleIo>(
        &mut self,
        io: &mut I,
        playback: Option<&PcmFormat>,
        microphone: Option<&PcmFormat>,
    ) -> Result<(), AudioError> {
        self.tick(io, &AudioRouter::default(), playback, microphone)
    }
}

#[derive(Clone, Debug)]
pub(super) struct View {
    pub endpoints: Vec<AudioEndpoint>,
    pub health: AudioDiagnostics,
    pub limitation: &'static str,
    pub activity: Activity,
    pub routing: RoutingState,
    pub playback_devices: Vec<HostDevice>,
    pub capture_devices: Vec<HostDevice>,
    pub routing_error: Option<String>,
    pub playback_channel_labels: Vec<String>,
    pub capture_channel_labels: Vec<String>,
}
pub(super) fn snapshot(audio: &ControllerAudio, activity: &Activity) -> View {
    let endpoints = audio.endpoints().to_vec();
    View {
        playback_channel_labels: endpoints
            .iter()
            .find(|endpoint| endpoint.direction() == SampleDirection::HostToController)
            .map(|endpoint| {
                endpoint
                    .format()
                    .channels()
                    .iter()
                    .map(|channel| format!("{channel:?}"))
                    .collect()
            })
            .unwrap_or_default(),
        capture_channel_labels: endpoints
            .iter()
            .find(|endpoint| endpoint.direction() == SampleDirection::ControllerToHost)
            .map(|endpoint| {
                endpoint
                    .format()
                    .channels()
                    .iter()
                    .map(|channel| format!("{channel:?}"))
                    .collect()
            })
            .unwrap_or_default(),
        endpoints,
        health: audio.diagnostics(),
        limitation: audio.limitation(),
        activity: Activity {
            pending: Vec::new(),
            ..activity.clone()
        },
        routing: activity.routing.clone(),
        playback_devices: activity.playback_devices.clone(),
        capture_devices: activity.capture_devices.clone(),
        routing_error: activity.routing_error.clone(),
    }
}
pub(super) fn cycle(
    audio: &mut ControllerAudio,
    activity: &mut Activity,
    router: &mut AudioRouter,
    action: Option<Action>,
) -> Result<(), AudioError> {
    if let Some(action) = action {
        if matches!(&action, Action::RetryRouting) {
            router.reset();
        }
        if matches!(&action, Action::Tone(true))
            && !audio.endpoints().iter().any(|e| {
                e.direction() == SampleDirection::ControllerToHost
                    && e.access() == AudioAccess::Samples
            })
        {
            return Err(AudioError::OwnershipMismatch);
        }
        activity.apply(audio, action)?;
    }
    let format = |direction| {
        audio
            .endpoints()
            .iter()
            .find(|endpoint| {
                endpoint.direction() == direction && endpoint.access() == AudioAccess::Samples
            })
            .map(|endpoint| endpoint.format().clone())
    };
    let playback = format(SampleDirection::HostToController);
    let microphone = format(SampleDirection::ControllerToHost);
    activity.routing_error =
        router.configure(&activity.routing, playback.is_some(), microphone.is_some());
    let result = activity.tick(audio, router, playback.as_ref(), microphone.as_ref());
    if let Some(error) = router.error() {
        activity.routing_error = Some(error);
    }
    result
}
fn error_category(error: &AudioError) -> &'static str {
    match error {
        AudioError::Closed => "Closed",
        AudioError::OwnershipMismatch => "Ownership mismatch",
        AudioError::InvalidSampleBuffer => "Invalid sample buffer",
        AudioError::Unsupported => "Unsupported audio request",
        AudioError::AccessDenied => "Audio permission denied",
        AudioError::Unavailable => "Audio unavailable",
        AudioError::InvalidRequirement | AudioError::IncompatibleTopology => "Audio configuration",
        AudioError::Backend { .. } => "Audio backend/worker",
        _ => "Audio error",
    }
}
pub(super) fn error_message(error: &AudioError) -> String {
    format!("{}: {error}", error_category(error))
}

fn selector_text(selector: &virtualgamepad::AudioEndpointSelector) -> String {
    if let Some(name) = selector.pipewire_node() {
        name.to_owned()
    } else if let Some((card, device, subdevice)) = selector.alsa_pcm() {
        format!("hw:CARD={card},DEV={device},SUBDEV={subdevice}")
    } else {
        format!("{selector:?}")
    }
}

pub(super) fn draw(ui: &mut egui::Ui, view: Option<&View>) -> Option<Action> {
    let mut action = None;
    ui.group(|ui| {
        ui.heading("Controller audio");
        let Some(view) = view else { ui.label("Audio disabled for this creation."); return; };
        ui.label(view.limitation);
        if let Some(error) = &view.routing_error {
            ui.colored_label(egui::Color32::YELLOW, format!("Host audio routing: {error}"));
            if ui.button("Retry host audio routing").clicked() {
                action = Some(Action::RetryRouting);
            }
        }
        ui.label(format!("Closed: {} · Failed: {} · Underrun frames: {} · Dropped playback: {}",
            view.health.is_closed(), view.health.failed(), view.health.underrun_frames(), view.health.dropped_playback_frames()));
        if let Some(error) = view.health.last_error() { ui.colored_label(egui::Color32::YELLOW, error_message(error)); }
        ui.label(format!("Playback monitor: {} frames, {} discontinuities, peak {} · Latest segment at {:?} · Microphone test: {} accepted frames",
            view.activity.playback_frames, view.activity.discontinuities, view.activity.peak, view.activity.first_playback_frame, view.activity.microphone_frames));
        ui.weak("Samples playback is monitored here. The meters and selectors below belong to the physical audio devices on this controller.");
        for endpoint in &view.endpoints {
            ui.collapsing(format!("{} · {:?} · {:?}", endpoint.group(), endpoint.direction(), endpoint.access()), |ui| {
                ui.label(format!("{} Hz · {:?}", endpoint.format().sample_rate_hz(), endpoint.format().channels()));
                ui.label(format!("Clock domain: {}", endpoint.clock_domain()));
                ui.horizontal_wrapped(|ui| {
                    let host = selector_text(endpoint.host());
                    ui.monospace(format!("Host: {host}"));
                    if ui.button("Copy host selector").clicked() { ui.ctx().copy_text(host); }
                });
                if let Some(caller) = endpoint.caller() {
                    ui.horizontal_wrapped(|ui| {
                        let caller = selector_text(caller);
                        ui.monospace(format!("Caller: {caller}"));
                        if ui.button("Copy caller selector").clicked() { ui.ctx().copy_text(caller); }
                    });
                }
                ui.weak("Selectors belong to this open creation; resolve anew after recreation.");
            });
        }
        let sample_owner = |direction| view.endpoints.iter().any(|e| e.direction() == direction && e.access() == AudioAccess::Samples);
        ui.add_enabled_ui(!view.health.is_closed() && !view.health.failed(), |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.add_enabled(sample_owner(SampleDirection::HostToController), egui::Button::new("Flush playback")).clicked() { action = Some(Action::FlushPlayback); }
                if ui.add_enabled(sample_owner(SampleDirection::ControllerToHost), egui::Button::new("Flush microphone")).clicked() { action = Some(Action::FlushMicrophone); }
                let mut tone = view.activity.tone;
                if ui.add_enabled(sample_owner(SampleDirection::ControllerToHost), egui::Checkbox::new(&mut tone, "440 Hz microphone test tone")).changed() { action = Some(Action::Tone(tone)); }
            });
        });
    });
    action
}

fn draw_device_selector(
    ui: &mut egui::Ui,
    id: &str,
    selected: &mut String,
    devices: &[HostDevice],
) -> bool {
    let selected_label = devices
        .iter()
        .find(|device| device.id == *selected)
        .map_or("Device unavailable", |device| device.label.as_str());
    let mut changed = false;
    egui::ComboBox::from_id_salt(id)
        .selected_text(selected_label)
        .width(ui.available_width())
        .show_ui(ui, |ui| {
            for device in devices {
                changed |= ui
                    .selectable_value(selected, device.id.clone(), &device.label)
                    .changed();
            }
        });
    changed
}

fn draw_meters(ui: &mut egui::Ui, label: &str, channels: &[u16]) {
    for (channel, peak) in channels.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.label(format!("{label} {}", channel + 1));
            let value = f32::from(*peak) / f32::from(u16::MAX);
            ui.add(egui::ProgressBar::new(value).desired_width(ui.available_width()));
        });
    }
}

#[allow(clippy::too_many_lines)]
pub(super) fn draw_output_routes(
    ui: &mut egui::Ui,
    routing: &mut RoutingState,
    devices: &[HostDevice],
    playback_channel_labels: &[String],
) -> bool {
    let mut changed = false;
    if let Some(connector) = routing.jack_connector {
        let plugged = ui.checkbox(
            &mut routing.jack_connected,
            format!(
                "{} mm jack plugged in",
                match connector {
                    JackConnector::Mm25 => "2.5",
                    JackConnector::Mm35 => "3.5",
                }
            ),
        );
        if plugged.changed() {
            routing.set_jack_connected(
                routing.jack_connected,
                AudioTopology {
                    onboard_speaker_channels: routing
                        .outputs
                        .iter()
                        .find(|route| route.id == "onboard-speaker")
                        .map_or(0, |route| route.channels),
                    onboard_microphone_channels: routing
                        .inputs
                        .iter()
                        .find(|route| route.id == "onboard-microphone")
                        .map_or(0, |route| route.target_channels.len()),
                    jack: routing.jack_connector,
                },
            );
            changed = true;
        }
        if routing.jack_connected {
            egui::ComboBox::from_id_salt("jack_device_type")
                .selected_text(
                    routing
                        .jack_connector
                        .map_or(routing.jack_device.label(), |connector| {
                            routing.jack_device.label_for_connector(connector)
                        }),
                )
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for device in JackDevice::ALL {
                        changed |= ui
                            .selectable_value(
                                &mut routing.jack_device,
                                device,
                                routing.jack_connector.map_or(device.label(), |connector| {
                                    device.label_for_connector(connector)
                                }),
                            )
                            .changed();
                    }
                });
            if changed {
                let topology = AudioTopology {
                    onboard_speaker_channels: routing
                        .outputs
                        .iter()
                        .find(|route| route.id == "onboard-speaker")
                        .map_or(0, |route| route.channels),
                    onboard_microphone_channels: routing
                        .inputs
                        .iter()
                        .find(|route| route.id == "onboard-microphone")
                        .map_or(0, |route| route.target_channels.len()),
                    jack: routing.jack_connector,
                };
                routing.set_jack_device(routing.jack_device, topology);
            }
        }
    }
    for route in &mut routing.outputs {
        ui.separator();
        changed |= ui.checkbox(&mut route.enabled, &route.label).changed();
        ui.label("Play through");
        changed |= draw_device_selector(ui, &route.id, &mut route.device_id, devices);
        ui.horizontal(|ui| {
            ui.label("Output channels");
            changed |= ui
                .add(egui::DragValue::new(&mut route.channels).range(1..=32))
                .changed();
        });
        if route.source_channels.len() != route.channels {
            route.source_channels.resize_with(route.channels, || None);
            for (source, default) in
                route
                    .source_channels
                    .iter_mut()
                    .zip(super::audio_topology::default_channel_routes(
                        playback_channel_labels.len(),
                        route.channels,
                    ))
            {
                if source.is_none() {
                    *source = default.source;
                }
            }
            route.peak.resize(route.channels, 0);
            changed = true;
        }
        for (channel, source) in route.source_channels.iter_mut().enumerate() {
            let selected = source
                .and_then(|index| playback_channel_labels.get(index))
                .map_or("Silence", String::as_str);
            egui::ComboBox::from_id_salt(("output_channel", &route.id, channel))
                .selected_text(selected)
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    changed |= ui.selectable_value(source, None, "Silence").changed();
                    for (index, label) in playback_channel_labels.iter().enumerate() {
                        changed |= ui.selectable_value(source, Some(index), label).changed();
                    }
                });
        }
        draw_meters(ui, "Channel", &route.peak);
    }
    changed
}

pub(super) fn draw_input_routes(
    ui: &mut egui::Ui,
    routing: &mut RoutingState,
    devices: &[HostDevice],
    capture_channel_labels: &[String],
) -> bool {
    let mut changed = false;
    for route in &mut routing.inputs {
        ui.separator();
        changed |= ui
            .checkbox(
                &mut route.enabled,
                format!("Pipe {} to controller", route.label),
            )
            .changed();
        ui.label("Audio source");
        changed |= draw_device_selector(ui, &route.id, &mut route.device_id, devices);
        ui.horizontal(|ui| {
            ui.label("Source channels");
            changed |= ui
                .add(egui::DragValue::new(&mut route.source_channels).range(1..=32))
                .changed();
        });
        if route.target_channels.len() != route.source_channels {
            route
                .target_channels
                .resize_with(route.source_channels, || None);
            for (destination, default) in
                route
                    .target_channels
                    .iter_mut()
                    .zip(super::audio_topology::default_channel_routes(
                        route.source_channels,
                        capture_channel_labels.len(),
                    ))
            {
                if destination.is_none() {
                    *destination = (default.source.is_some()).then_some(default.destination);
                }
            }
            route.peak.resize(route.source_channels, 0);
            changed = true;
        }
        for (channel, destination) in route.target_channels.iter_mut().enumerate() {
            let selected = destination
                .and_then(|index| capture_channel_labels.get(index))
                .map_or("Silence", String::as_str);
            egui::ComboBox::from_id_salt(("input_channel", &route.id, channel))
                .selected_text(selected)
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    changed |= ui.selectable_value(destination, None, "Silence").changed();
                    for (index, label) in capture_channel_labels.iter().enumerate() {
                        changed |= ui
                            .selectable_value(destination, Some(index), label)
                            .changed();
                    }
                });
        }
        draw_meters(ui, "Channel", &route.peak);
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use virtualgamepad::AudioChannel;

    #[test]
    fn audio_disabled_controller_skips_host_device_discovery() {
        let activity = Activity::default();
        assert!(!activity.initialized);
        assert!(activity.playback_devices.is_empty());
        assert!(activity.capture_devices.is_empty());
        assert!(activity.routing.outputs.is_empty());
        assert!(activity.routing.inputs.is_empty());
    }

    #[test]
    fn native_audio_owners_do_not_start_sample_routes() {
        let topology = AudioTopology::for_family(ControllerFamily::DualSense);
        let mut routing = RoutingState::for_topology(topology, HostBackend::PipeWire);
        routing.inputs[0].enabled = true;
        let (playback, capture, errors) = sample_route_ownership(&routing, false, true);
        assert!(!playback);
        assert!(capture);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("playback as a native client"));

        let (playback, capture, errors) = sample_route_ownership(&routing, true, false);
        assert!(playback);
        assert!(!capture);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("microphone capture as a native client"));
    }

    #[test]
    fn per_controller_routes_follow_onboard_and_jack_state() {
        let topology = AudioTopology::for_family(ControllerFamily::DualSense);
        let mut state = RoutingState::for_topology(topology, HostBackend::PipeWire);
        assert_eq!(
            state
                .outputs
                .iter()
                .map(|route| route.id.as_str())
                .collect::<Vec<_>>(),
            ["onboard-speaker"]
        );
        assert_eq!(
            state
                .inputs
                .iter()
                .map(|route| route.id.as_str())
                .collect::<Vec<_>>(),
            ["onboard-microphone"]
        );
        assert!(!state.inputs[0].enabled);
        state.set_jack_connected(true, topology);
        assert_eq!(state.outputs.len(), 2);
        assert_eq!(state.inputs.len(), 2);
        assert!(state.inputs[1].label.contains("TRRS"));
        state.set_jack_device(JackDevice::Microphone, topology);
        assert_eq!(state.outputs.len(), 1);
        assert_eq!(state.inputs.len(), 2);
        state.set_jack_connected(false, topology);
        assert_eq!(state.outputs.len(), 1);
        assert_eq!(state.inputs.len(), 1);
    }

    #[test]
    fn xbox_headset_label_tracks_its_physical_connector() {
        assert_eq!(
            JackDevice::Headset.label_for_connector(JackConnector::Mm25),
            "Wired headset"
        );
        assert_eq!(
            JackDevice::Headset.label_for_connector(JackConnector::Mm35),
            "TRRS headset"
        );
    }

    #[test]
    fn creation_controls_fit_narrow_sidebar_without_horizontal_overflow() {
        let context = egui::Context::default();
        let mut config = CreationAudio {
            enabled: true,
            ..CreationAudio::default()
        };
        let mut rect = egui::Rect::NOTHING;
        let available_width = 192.0;
        let _ = context.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(available_width, 480.0),
                )),
                ..Default::default()
            },
            |context| {
                egui::CentralPanel::default().show(context, |ui| {
                    ui.set_width(available_width);
                    rect = draw_creation(ui, &mut config, RealizationId::LINUX_UHID_USB, true);
                });
            },
        );
        assert!(rect.width() <= available_width + 1.0, "{rect:?}");
    }

    #[test]
    fn audio_creation_choices_remain_visible_when_backend_is_unavailable() {
        let context = egui::Context::default();
        let mut config = CreationAudio {
            enabled: true,
            ..CreationAudio::default()
        };
        let mut rect = egui::Rect::NOTHING;
        let _ = context.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(240.0, 480.0),
                )),
                ..Default::default()
            },
            |context| {
                egui::CentralPanel::default().show(context, |ui| {
                    rect = draw_creation(ui, &mut config, RealizationId::LINUX_UINPUT, true);
                });
            },
        );

        assert!(rect.height() > 80.0, "audio choices were hidden: {rect:?}");
    }

    #[derive(Default)]
    struct Fake {
        reads: usize,
        writes: Vec<Vec<i16>>,
        flushes: Vec<SampleDirection>,
        accepted: usize,
        fail: bool,
    }
    impl SampleIo for Fake {
        fn read(&mut self, dest: &mut [i16]) -> Result<ReadObservation, AudioError> {
            self.reads += 1;
            if self.fail {
                return Err(AudioError::Backend {
                    reason: "synthetic terminal failure".into(),
                });
            }
            dest[..2].copy_from_slice(&[7, -9]);
            Ok(ReadObservation {
                first_frame: self.reads as u64,
                frames: 1,
                discontinuity: true,
            })
        }
        fn write(&mut self, samples: &[i16]) -> Result<usize, AudioError> {
            if self.fail {
                return Err(AudioError::Backend {
                    reason: "synthetic microphone failure".into(),
                });
            }
            self.writes.push(samples.to_vec());
            Ok(self.accepted.min(samples.len()))
        }
        fn flush(&mut self, direction: SampleDirection) -> Result<(), AudioError> {
            self.flushes.push(direction);
            Ok(())
        }
    }
    #[test]
    fn microphone_partial_writes_retry_suffix_and_stop_flushes_only_microphone() {
        let format = PcmFormat::new(48_000, &[AudioChannel::Microphone]).unwrap();
        let mut fake = Fake {
            accepted: 13,
            ..Fake::default()
        };
        let mut activity = Activity::default();
        activity.apply(&mut fake, Action::Tone(true)).unwrap();
        activity
            .tick_without_routes(&mut fake, None, Some(&format))
            .unwrap();
        activity
            .tick_without_routes(&mut fake, None, Some(&format))
            .unwrap();
        assert_eq!(fake.writes[0].len(), FRAMES);
        assert_eq!(fake.writes[1], fake.writes[0][13..]);
        assert_eq!(activity.microphone_frames, 26);
        activity.apply(&mut fake, Action::FlushPlayback).unwrap();
        assert_eq!(activity.pending.len(), FRAMES);
        activity.apply(&mut fake, Action::Tone(false)).unwrap();
        activity
            .tick_without_routes(&mut fake, None, Some(&format))
            .unwrap();
        assert_eq!(fake.writes.len(), 2);
        assert_eq!(
            fake.flushes,
            [
                SampleDirection::HostToController,
                SampleDirection::ControllerToHost
            ]
        );
        assert!(activity.pending.is_empty());
    }
    #[test]
    fn native_owners_do_not_call_sample_io_and_failures_are_typed() {
        let mut fake = Fake::default();
        let mut activity = Activity::default();
        activity.tick_without_routes(&mut fake, None, None).unwrap();
        assert_eq!(fake.reads, 0);
        assert!(fake.writes.is_empty());
        fake.fail = true;
        let format = PcmFormat::new(48_000, &[AudioChannel::AudibleLeft]).unwrap();
        let error = activity
            .tick_without_routes(&mut fake, Some(&format), None)
            .unwrap_err();
        assert!(error_message(&error).starts_with("Audio backend/worker:"));
    }
    #[test]
    fn playback_observations_are_bounded_complete_frames() {
        let format = PcmFormat::new(
            48_000,
            &[AudioChannel::AudibleLeft, AudioChannel::AudibleRight],
        )
        .unwrap();
        let mut fake = Fake::default();
        let mut activity = Activity::default();
        for _ in 0..64 {
            activity
                .tick_without_routes(&mut fake, Some(&format), None)
                .unwrap();
        }
        assert_eq!(
            (
                activity.playback_frames,
                activity.discontinuities,
                activity.peak
            ),
            (64, 64, 9)
        );
        assert_eq!(activity.first_playback_frame, Some(64));
        assert_eq!(fake.reads, 64);
        assert!(fake.writes.is_empty());
    }
    #[test]
    fn microphone_failure_preserves_pending_suffix_without_claiming_delivery() {
        let format = PcmFormat::new(48_000, &[AudioChannel::Microphone]).unwrap();
        let mut fake = Fake {
            fail: true,
            ..Fake::default()
        };
        let mut activity = Activity::default();
        activity.apply(&mut fake, Action::Tone(true)).unwrap();
        assert!(
            activity
                .tick_without_routes(&mut fake, None, Some(&format))
                .is_err()
        );
        assert_eq!(activity.microphone_frames, 0);
        assert_eq!(activity.pending.len(), FRAMES);
    }
    #[test]
    fn typed_selector_copy_uses_the_creation_scoped_target() {
        assert_eq!(
            selector_text(&virtualgamepad::AudioEndpointSelector::PipeWireNode {
                name: "synthetic.node.1".into()
            }),
            "synthetic.node.1"
        );
        assert_eq!(
            selector_text(&virtualgamepad::AudioEndpointSelector::AlsaPcm {
                card_id: "synthetic".into(),
                device: 2,
                subdevice: 3
            }),
            "hw:CARD=synthetic,DEV=2,SUBDEV=3"
        );
    }
    #[test]
    fn creation_feature_and_ownership_matrix_is_explicit() {
        for target in [
            RealizationId::LINUX_UHID_USB,
            RealizationId::LINUX_USBIP_USB_AUDIO,
        ] {
            for playback in [AudioAccess::Samples, AudioAccess::NativeClient] {
                for microphone in [AudioAccess::Samples, AudioAccess::NativeClient] {
                    let config = CreationAudio {
                        enabled: true,
                        playback,
                        microphone,
                        host_backend: default_host_backend(),
                    };
                    let native_usb = target == RealizationId::LINUX_USBIP_USB_AUDIO
                        && (playback == AudioAccess::NativeClient
                            || microphone == AudioAccess::NativeClient);
                    let expected = backend_available(target)
                        && (!native_usb || cfg!(feature = "audio-pipewire"));
                    assert_eq!(config.options(target, true).is_ok(), expected);
                    assert!(config.options(target, false).is_err());
                }
            }
        }
    }
    #[test]
    fn unsupported_creation_choices_fail_before_factories() {
        for has_audio in [false, true] {
            assert!(
                CreationAudio {
                    enabled: true,
                    ..CreationAudio::default()
                }
                .options(RealizationId::LINUX_UINPUT, has_audio)
                .is_err()
            );
        }
        let target = default_creation_target();
        let defaults = CreationAudio::default();
        assert_eq!(defaults.enabled, default_audio_enabled(target, true));
        assert_eq!(defaults.playback, AudioAccess::Samples);
        assert_eq!(defaults.microphone, AudioAccess::Samples);
        assert!(defaults.options(target, true).is_ok());
        let usbip_audio = CreationAudio {
            enabled: default_audio_enabled(RealizationId::LINUX_USBIP_USB_AUDIO, true),
            ..defaults
        };
        assert_eq!(
            usbip_audio
                .options(RealizationId::LINUX_USBIP_USB_AUDIO, true)
                .is_ok(),
            cfg!(all(target_os = "linux", feature = "audio-usbip"))
        );
    }
}
