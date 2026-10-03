//! Demo-owned bounded PCM work; only the root application API is used.
use super::audio_topology::{AudioTopology, ControllerFamily, JackConnector, JackDevice};
use eframe::egui;
use std::collections::HashMap;
use virtualgamepad::{
    AudioAccess, AudioChannel, AudioDiagnostics, AudioEndpoint, AudioError, AudioExposure,
    AudioOptions, ControllerAudio, ControllerError, CreationOptions, PcmFormat, RealizationId,
    SampleDirection,
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
    outputs: HashMap<String, Vec<RoutedOutputPort>>,
    inputs: HashMap<String, Vec<RoutedInputPort>>,
    last_error: Option<String>,
}

struct RoutedOutputPort {
    device_channels: Vec<usize>,
    port: super::host_audio::OutputPort,
}

struct RoutedInputPort {
    device_channels: Vec<usize>,
    port: super::host_audio::InputPort,
}

trait HostSampleRouter {
    fn output(&self, id: &str, samples: &[i16], channels: usize) -> bool;
    fn input(&self, id: &str, channels: usize) -> Option<Vec<i16>>;
}

impl HostSampleRouter for AudioRouter {
    fn output(&self, id: &str, samples: &[i16], channels: usize) -> bool {
        self.output(id, samples, channels)
    }

    fn input(&self, id: &str, channels: usize) -> Option<Vec<i16>> {
        self.input(id, channels)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct OutputDeviceGroup {
    device_id: String,
    channels: Vec<usize>,
}

fn output_device_groups(route: &OutputRoute) -> Vec<OutputDeviceGroup> {
    let mut groups: Vec<OutputDeviceGroup> = Vec::new();
    for channel in 0..route.channels {
        let device_id = route
            .channel_device_ids
            .get(channel)
            .cloned()
            .unwrap_or_else(|| "default".into());
        if let Some(group) = groups.iter_mut().find(|group| group.device_id == device_id) {
            group.channels.push(channel);
        } else {
            groups.push(OutputDeviceGroup {
                device_id,
                channels: vec![channel],
            });
        }
    }
    groups
}

fn input_device_groups(route: &InputRoute) -> Vec<OutputDeviceGroup> {
    let mut groups: Vec<OutputDeviceGroup> = Vec::new();
    for channel in 0..route.source_channels {
        let device_id = route
            .channel_device_ids
            .get(channel)
            .cloned()
            .unwrap_or_else(|| "default".into());
        if let Some(group) = groups.iter_mut().find(|group| group.device_id == device_id) {
            group.channels.push(channel);
        } else {
            groups.push(OutputDeviceGroup {
                device_id,
                channels: vec![channel],
            });
        }
    }
    groups
}

fn collect_input_channels(channels: usize, groups: &[(Vec<usize>, Vec<i16>)]) -> Option<Vec<i16>> {
    if channels == 0 {
        return None;
    }
    let frames = groups.iter().find_map(|(device_channels, samples)| {
        (!device_channels.is_empty()).then_some(samples.len() / device_channels.len())
    })?;
    let mut combined = vec![0; frames * channels];
    let mut copied = false;
    for (device_channels, samples) in groups {
        if device_channels.is_empty() {
            continue;
        }
        let group_frames = (samples.len() / device_channels.len()).min(frames);
        for frame in 0..group_frames {
            for (source_channel, destination_channel) in device_channels.iter().copied().enumerate()
            {
                if destination_channel < channels {
                    combined[frame * channels + destination_channel] =
                        samples[frame * device_channels.len() + source_channel];
                    copied = true;
                }
            }
        }
    }
    copied.then_some(combined)
}

fn select_output_channels(samples: &[i16], source_channels: usize, channels: &[usize]) -> Vec<i16> {
    if source_channels == 0 || channels.is_empty() {
        return Vec::new();
    }
    let frames = samples.len() / source_channels;
    let mut selected = Vec::with_capacity(frames * channels.len());
    for frame in 0..frames {
        for &channel in channels {
            selected.push(
                samples
                    .get(frame * source_channels + channel)
                    .copied()
                    .unwrap_or_default(),
            );
        }
    }
    selected
}

impl AudioRouter {
    fn configure(
        &mut self,
        routing: &RoutingState,
        sample_playback: bool,
        sample_capture: bool,
    ) -> Option<String> {
        let config = (routing.configuration(), sample_playback, sample_capture);
        if self.config.as_ref() == Some(&config) {
            return self.last_error.clone();
        }
        self.outputs.clear();
        self.inputs.clear();
        self.config = Some(config);
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
            let mut ports = Vec::new();
            for group in output_device_groups(output) {
                match super::host_audio::OutputPort::open(
                    routing.backend,
                    &group.device_id,
                    group.channels.len(),
                ) {
                    Ok(port) => ports.push(RoutedOutputPort {
                        device_channels: group.channels,
                        port,
                    }),
                    Err(error) => errors.push(format!(
                        "{} channels {:?}: {error}",
                        output.label, group.channels
                    )),
                }
            }
            if !ports.is_empty() {
                self.outputs.insert(output.id.clone(), ports);
            }
        }
        for input in routing
            .inputs
            .iter()
            .filter(|route| route.connected && route.enabled && allow_capture)
        {
            let mut ports = Vec::new();
            for group in input_device_groups(input) {
                match super::host_audio::InputPort::open(
                    routing.backend,
                    &group.device_id,
                    group.channels.len(),
                ) {
                    Ok(port) => ports.push(RoutedInputPort {
                        device_channels: group.channels,
                        port,
                    }),
                    Err(error) => errors.push(format!(
                        "{} channels {:?}: {error}",
                        input.label, group.channels
                    )),
                }
            }
            if !ports.is_empty() {
                self.inputs.insert(input.id.clone(), ports);
            }
        }
        self.last_error = (!errors.is_empty()).then(|| errors.join("; "));
        self.last_error.clone()
    }

    fn output(&self, id: &str, samples: &[i16], channels: usize) -> bool {
        self.outputs.get(id).is_some_and(|ports| {
            ports.iter().fold(true, |written, route| {
                let selected = select_output_channels(samples, channels, &route.device_channels);
                route.port.write(&selected) && written
            })
        })
    }

    fn input(&self, id: &str, channels: usize) -> Option<Vec<i16>> {
        let ports = self.inputs.get(id)?;
        let mut blocks = Vec::with_capacity(ports.len());
        for route in ports {
            let mut newest = route.port.read();
            while let Some(next) = route.port.read() {
                newest = Some(next);
            }
            if let Some(samples) = newest {
                blocks.push((route.device_channels.clone(), samples));
            }
        }
        collect_input_channels(channels, &blocks)
    }

    fn error(&self) -> Option<String> {
        let mut errors: Vec<_> =
            self.outputs
                .iter()
                .flat_map(|(id, ports)| {
                    let id = id.clone();
                    ports.iter().filter_map(move |route| {
                        route.port.error().map(|error| {
                            format!("{id} channels {:?}: {error}", route.device_channels)
                        })
                    })
                })
                .chain(self.inputs.iter().flat_map(|(id, ports)| {
                    let id = id.clone();
                    ports.iter().filter_map(move |route| {
                        route.port.error().map(|error| {
                            format!("{id} channels {:?}: {error}", route.device_channels)
                        })
                    })
                }))
                .collect();
        let dropped: u64 = self
            .outputs
            .values()
            .flat_map(|ports| ports.iter())
            .map(|route| route.port.dropped_packets())
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
    pub connected: bool,
    pub enabled: bool,
    pub channel_device_ids: Vec<String>,
    pub manual_channel_control: bool,
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
    pub channel_device_ids: Vec<String>,
    pub manual_channel_control: bool,
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
    fn configuration(&self) -> Self {
        let mut config = self.clone();
        // Meter observations change each cycle without changing host streams.
        for route in &mut config.outputs {
            route.peak.clear();
        }
        for route in &mut config.inputs {
            route.peak.clear();
        }
        config
    }

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
                channel_device_ids: vec!["default".into(); topology.onboard_speaker_channels],
                manual_channel_control: false,
                source_channels: vec![None; topology.onboard_speaker_channels],
                peak: vec![0; topology.onboard_speaker_channels],
            });
        }
        if topology.onboard_microphone_channels > 0 {
            state.inputs.push(InputRoute {
                id: "onboard-microphone".into(),
                label: "Built-in microphone".into(),
                connected: true,
                enabled: false,
                channel_device_ids: vec!["default".into(); topology.onboard_microphone_channels],
                manual_channel_control: false,
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
                channel_device_ids: vec!["default".into(); channels],
                manual_channel_control: false,
                source_channels: (0..channels).map(Some).collect(),
                peak: vec![0; channels],
            });
        }
        if let Some(connector) = topology.jack {
            let microphone_connected = connected && self.jack_device.input_channels() > 0;
            let channels = if microphone_connected {
                self.jack_device.input_channels()
            } else {
                JackDevice::Headset.input_channels()
            };
            let label = if microphone_connected && self.jack_device == JackDevice::Microphone {
                "Jack microphone".to_owned()
            } else if microphone_connected {
                format!(
                    "{} microphone",
                    self.jack_device.label_for_connector(connector)
                )
            } else {
                format!(
                    "{} microphone",
                    JackDevice::Headset.label_for_connector(connector)
                )
            };
            self.inputs.push(InputRoute {
                id: "jack-microphone".into(),
                label,
                connected: microphone_connected,
                enabled: false,
                channel_device_ids: vec!["default".into(); channels],
                manual_channel_control: false,
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

    fn select_jack_device(&mut self, device: Option<JackDevice>, topology: AudioTopology) {
        match device {
            Some(device) => {
                self.set_jack_device(device, topology);
                self.set_jack_connected(true, topology);
            }
            None => self.set_jack_connected(false, topology),
        }
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

pub(super) const fn controller_audio_enabled(config: CreationAudio) -> bool {
    config.enabled
}

fn backend_status(target: RealizationId, has_audio: bool) -> Option<&'static str> {
    if !has_audio {
        Some("Unsupported for this family")
    } else if target == RealizationId::LINUX_UHID_USB
        && !cfg!(all(target_os = "linux", feature = "audio-pipewire"))
    {
        Some("Build with --features audio-pipewire")
    } else if target == RealizationId::LINUX_USBIP_USB_AUDIO
        && !cfg!(all(target_os = "linux", feature = "audio-usbip"))
    {
        Some("Build with --features audio-usbip")
    } else if !backend_available(target) {
        Some("Audio requires UHID or USB/IP")
    } else {
        None
    }
}

impl CreationAudio {
    pub fn options(
        self,
        target: RealizationId,
        has_audio: bool,
    ) -> Result<CreationOptions, ControllerError> {
        if self.enabled && (!has_audio || !backend_available(target)) {
            let reason = backend_status(target, has_audio)
                .unwrap_or("Emulated audio is unavailable for this creation choice");
            return Err(ControllerError::Unsupported {
                reason: format!("{reason}. Select another creation choice or disable audio."),
            });
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
            .width(ui.available_width())
            .show_ui(ui, add_options);
    });
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

        let status = if let Some(status) = backend_status(target, has_audio) {
            Some(status)
        } else if target == RealizationId::LINUX_USBIP_USB_AUDIO && !config.enabled {
            Some("USB/IP requires audio")
        } else {
            None
        };
        if let Some(status) = status {
            ui.colored_label(egui::Color32::YELLOW, status);
        }

        if has_audio {
            let available = config.enabled && backend_available(target);
            ui.add_enabled_ui(available, |ui| {
                ui.vertical(|ui| {
                    draw_labeled_combo_box(
                        ui,
                        "Host audio",
                        "host_audio_backend",
                        config.host_backend.label(),
                        true,
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
                            true,
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
                });
            });
        }
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
const AUDIO_DEVICE_SELECTOR_WIDTH: f32 = 135.0;
const JACK_DEVICE_SELECTOR_WIDTH: f32 = 210.0;
const AUDIO_VU_WIDTH: f32 = 120.0;
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
    fn tick<I: SampleIo, R: HostSampleRouter>(
        &mut self,
        io: &mut I,
        router: &R,
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
                    let _ = router.output(&route.id, &samples, route.channels);
                }
            }
        }
        if let Some(format) = microphone {
            let channels = format.channels().len();
            let mut microphone_samples = Vec::<i16>::new();
            let mut has_input = false;
            for route in &mut self.routing.inputs {
                if !route.enabled {
                    route.peak.fill(0);
                    continue;
                }
                let Some(source) = router.input(&route.id, route.source_channels) else {
                    continue;
                };
                let source_channels = route.source_channels.clamp(1, 32);
                let samples = (source.len() / source_channels).min(FRAMES) * channels;
                microphone_samples.resize(microphone_samples.len().max(samples), 0);
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
                microphone_samples.resize(FRAMES * channels, 0);
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
    update_provider_speaker_mapping(activity, audio);
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

pub(super) fn draw_diagnostics(ui: &mut egui::Ui, view: Option<&View>) -> Option<Action> {
    let mut action = None;
    draw_audio_info_header(ui, view);
    let Some(view) = view else {
        ui.weak("Waiting for controller audio status…");
        return None;
    };

    let status = if view.health.failed() {
        "Failed"
    } else if view.health.is_closed() {
        "Closed"
    } else {
        "Open"
    };
    draw_diagnostic_metric_rows(
        ui,
        &[
            ("Status", status.to_owned()),
            ("Underrun frames", view.health.underrun_frames().to_string()),
            (
                "Dropped playback",
                view.health.dropped_playback_frames().to_string(),
            ),
            ("Playback frames", view.activity.playback_frames.to_string()),
            ("Discontinuities", view.activity.discontinuities.to_string()),
            ("Playback peak", view.activity.peak.to_string()),
            (
                "Latest segment",
                view.activity
                    .first_playback_frame
                    .map_or_else(|| "—".to_owned(), |frame| frame.to_string()),
            ),
            (
                "Microphone test",
                format!("{} accepted frames", view.activity.microphone_frames),
            ),
        ],
    );
    ui.horizontal(|ui| {
        if let Some(error) = &view.routing_error {
            ui.colored_label(egui::Color32::YELLOW, "Host routing: error")
                .on_hover_text(error);
        } else {
            ui.label("Host routing: ready");
        }
        if ui
            .add_enabled(
                view.routing_error.is_some(),
                egui::Button::new("Retry routing"),
            )
            .clicked()
        {
            action = Some(Action::RetryRouting);
        }
    });
    if let Some(error) = view.health.last_error() {
        ui.colored_label(egui::Color32::YELLOW, error_message(error));
    }

    draw_endpoint_details(ui, &view.endpoints);
    draw_audio_actions(ui, view).or(action)
}

fn draw_audio_info_header(ui: &mut egui::Ui, view: Option<&View>) {
    ui.horizontal(|ui| {
        let info = ui.add_sized([18.0, 18.0], egui::Button::new("!"));
        if info.hovered() {
            egui::Tooltip::for_widget(&info).at_pointer().show(|ui| {
                ui.set_max_width(360.0);
                ui.strong("Controller audio details");
                if let Some(view) = view {
                    ui.label(view.limitation);
                }
                ui.label("Playback samples are monitored here. The meters and device selectors in Output and Input belong to this controller's physical audio endpoints.");
                ui.label("Endpoint selectors belong to this open creation; resolve them again after recreating the controller.");
            });
        }
    });
}

fn draw_endpoint_details(ui: &mut egui::Ui, endpoints: &[AudioEndpoint]) {
    ui.collapsing(format!("Endpoints ({})", endpoints.len()), |ui| {
        for endpoint in endpoints {
            ui.collapsing(
                format!(
                    "{} · {:?} · {:?}",
                    endpoint.group(),
                    endpoint.direction(),
                    endpoint.access()
                ),
                |ui| {
                    ui.label(format!(
                        "{} Hz · {:?}",
                        endpoint.format().sample_rate_hz(),
                        endpoint.format().channels()
                    ));
                    ui.label(format!("Clock domain: {}", endpoint.clock_domain()));
                    ui.horizontal_wrapped(|ui| {
                        let host = selector_text(endpoint.host());
                        ui.monospace(format!("Host: {host}"));
                        if ui.button("Copy host selector").clicked() {
                            ui.ctx().copy_text(host);
                        }
                    });
                    if let Some(caller) = endpoint.caller() {
                        ui.horizontal_wrapped(|ui| {
                            let caller = selector_text(caller);
                            ui.monospace(format!("Caller: {caller}"));
                            if ui.button("Copy caller selector").clicked() {
                                ui.ctx().copy_text(caller);
                            }
                        });
                    }
                },
            );
        }
    });
}

fn draw_audio_actions(ui: &mut egui::Ui, view: &View) -> Option<Action> {
    let mut action = None;
    let sample_owner = |direction| {
        view.endpoints.iter().any(|endpoint| {
            endpoint.direction() == direction && endpoint.access() == AudioAccess::Samples
        })
    };
    ui.add_enabled_ui(!view.health.is_closed() && !view.health.failed(), |ui| {
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    sample_owner(SampleDirection::HostToController),
                    egui::Button::new("Flush playback"),
                )
                .clicked()
            {
                action = Some(Action::FlushPlayback);
            }
            if ui
                .add_enabled(
                    sample_owner(SampleDirection::ControllerToHost),
                    egui::Button::new("Flush microphone"),
                )
                .clicked()
            {
                action = Some(Action::FlushMicrophone);
            }
            let mut tone = view.activity.tone;
            if ui
                .add_enabled(
                    sample_owner(SampleDirection::ControllerToHost),
                    egui::Checkbox::new(&mut tone, "440 Hz microphone test tone"),
                )
                .changed()
            {
                action = Some(Action::Tone(tone));
            }
        });
    });
    action
}

fn draw_diagnostic_metric_rows(ui: &mut egui::Ui, rows: &[(&str, String)]) -> egui::Rect {
    egui::Grid::new("controller_audio_metrics")
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            for (label, value) in rows {
                ui.label(*label);
                super::boxed_metric_output(ui, value);
                ui.end_row();
            }
        })
        .response
        .rect
}

fn draw_device_selector_sized(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash,
    selected: &mut String,
    devices: &[HostDevice],
    width: f32,
) -> bool {
    let selected_label = devices
        .iter()
        .find(|device| device.id == *selected)
        .map_or("Device unavailable", |device| device.label.as_str());
    let width = width.min(ui.available_width());
    let mut changed = false;
    ui.allocate_ui(egui::vec2(width, ui.spacing().interact_size.y), |ui| {
        egui::ComboBox::from_id_salt(id)
            .selected_text(selected_label)
            .wrap_mode(egui::TextWrapMode::Truncate)
            .width(width)
            .show_ui(ui, |ui| {
                for device in devices {
                    changed |= ui
                        .selectable_value(selected, device.id.clone(), &device.label)
                        .changed();
                }
            });
    });
    changed
}

#[allow(clippy::too_many_lines)]
pub(super) fn draw_output_routes(
    ui: &mut egui::Ui,
    routing: &mut RoutingState,
    devices: &[HostDevice],
    playback_channel_labels: &[String],
) -> bool {
    let mut changed = false;
    let topology = routing_topology(routing);

    for route in routing
        .outputs
        .iter_mut()
        .filter(|route| route.id == "onboard-speaker")
    {
        route.enabled = true;
        changed |= draw_output_route_channels(ui, route, devices, playback_channel_labels);
    }

    if let Some(connector) = routing.jack_connector {
        if routing
            .outputs
            .iter()
            .any(|route| route.id == "onboard-speaker")
        {
            ui.separator();
        }
        let selected_device = routing.jack_connected.then_some(routing.jack_device);
        let mut next_device = selected_device;
        let mut next_manual_control = routing
            .outputs
            .iter()
            .find(|route| route.id == "jack-output")
            .is_some_and(|route| route.manual_channel_control);
        ui.horizontal(|ui| {
            ui.label(format!("{} mm jack", connector.millimeters()));
            let width = JACK_DEVICE_SELECTOR_WIDTH.min(ui.available_width());
            ui.allocate_ui(egui::vec2(width, ui.spacing().interact_size.y), |ui| {
                egui::ComboBox::from_id_salt("jack_device_type")
                    .selected_text(next_device.map_or("Not plugged in", |device| {
                        device.label_for_connector(connector)
                    }))
                    .wrap_mode(egui::TextWrapMode::Truncate)
                    .width(width)
                    .show_ui(ui, |ui| {
                        changed |= ui
                            .selectable_value(&mut next_device, None, "Not plugged in")
                            .changed();
                        for device in JackDevice::ALL {
                            changed |= ui
                                .selectable_value(
                                    &mut next_device,
                                    Some(device),
                                    device.label_for_connector(connector),
                                )
                                .changed();
                        }
                    });
            });
            if next_device.is_some_and(|device| device.output_channels() > 1) {
                changed |= ui
                    .checkbox(&mut next_manual_control, "Override")
                    .on_hover_text(
                        "Allow the paired left and right output channels to use separate devices",
                    )
                    .changed();
            } else {
                next_manual_control = false;
            }
        });
        if next_device != selected_device {
            routing.select_jack_device(next_device, topology);
            changed = true;
        }
        if let Some(route) = routing
            .outputs
            .iter_mut()
            .find(|route| route.id == "jack-output")
            .filter(|route| route.manual_channel_control != next_manual_control)
        {
            route.manual_channel_control = next_manual_control;
            if !next_manual_control {
                for (left, right) in output_stereo_channel_pairs(route, playback_channel_labels) {
                    let selected = route.channel_device_ids[left].clone();
                    route.channel_device_ids[right].clone_from(&selected);
                }
            }
            changed = true;
        }

        if routing.jack_connected {
            for route in routing
                .outputs
                .iter_mut()
                .filter(|route| route.id == "jack-output")
            {
                ui.indent("jack_output_channels", |ui| {
                    changed |=
                        draw_output_route_channels(ui, route, devices, playback_channel_labels);
                });
            }
        }
    }
    changed
}

fn routing_topology(routing: &RoutingState) -> AudioTopology {
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
    }
}

fn semantic_channel_index(channels: &[AudioChannel], source: AudioChannel) -> Option<usize> {
    channels.iter().position(|channel| *channel == source)
}

fn update_provider_speaker_mapping(activity: &mut Activity, audio: &ControllerAudio) {
    let source = audio.onboard_speaker_source().and_then(|source| {
        audio
            .endpoints()
            .iter()
            .find(|endpoint| endpoint.direction() == SampleDirection::HostToController)
            .and_then(|endpoint| semantic_channel_index(endpoint.format().channels(), source))
    });
    for route in activity
        .routing
        .outputs
        .iter_mut()
        .filter(|route| route.id == "onboard-speaker")
    {
        route.source_channels.fill(source);
    }
}

fn output_channel_label(route: &OutputRoute, channel: usize, labels: &[String]) -> String {
    route
        .source_channels
        .get(channel)
        .copied()
        .flatten()
        .and_then(|index| labels.get(index))
        .map_or_else(|| "unmapped".to_owned(), Clone::clone)
}

fn output_stereo_channel_pairs(route: &OutputRoute, labels: &[String]) -> Vec<(usize, usize)> {
    super::audio_topology::stereo_channel_pairs(labels)
        .into_iter()
        .filter_map(|(left_source, right_source)| {
            let left = route
                .source_channels
                .iter()
                .position(|source| *source == Some(left_source))?;
            let right = route
                .source_channels
                .iter()
                .position(|source| *source == Some(right_source))?;
            Some((left, right))
        })
        .collect()
}

fn draw_output_route_channels(
    ui: &mut egui::Ui,
    route: &mut OutputRoute,
    devices: &[HostDevice],
    labels: &[String],
) -> bool {
    route
        .channel_device_ids
        .resize(route.channels, "default".into());
    let pairs = output_stereo_channel_pairs(route, labels);
    let mut changed = false;

    if !route.manual_channel_control {
        for &(left, right) in &pairs {
            if route.channel_device_ids[left] != route.channel_device_ids[right] {
                let selected = route.channel_device_ids[left].clone();
                route.channel_device_ids[right].clone_from(&selected);
                changed = true;
            }
        }
    }

    for channel in 0..route.channels {
        let paired_with = pairs.iter().find_map(|&(left, right)| {
            if channel == left {
                Some((right, true))
            } else if channel == right {
                Some((left, false))
            } else {
                None
            }
        });
        let is_leader = paired_with.is_some_and(|(_, leader)| leader);
        let is_secondary = paired_with.is_some_and(|(_, leader)| !leader);
        let source = output_channel_label(route, channel, labels);
        let mut device_changed = false;
        ui.horizontal(|ui| {
            let label = if route.id == "onboard-speaker" {
                format!("Onboard speaker ({source}):")
            } else {
                format!("Channel {} ({source}):", channel + 1)
            };
            ui.add_sized(
                [170.0, ui.spacing().interact_size.y],
                egui::Label::new(label).truncate(),
            );
            ui.add_enabled_ui(route.manual_channel_control || !is_secondary, |ui| {
                device_changed = draw_device_selector_sized(
                    ui,
                    (route.id.as_str(), channel),
                    &mut route.channel_device_ids[channel],
                    devices,
                    AUDIO_DEVICE_SELECTOR_WIDTH,
                );
            });
            let meter_width = ui.available_width().clamp(0.0, AUDIO_VU_WIDTH);
            ui.add_sized(
                [meter_width, ui.spacing().interact_size.y],
                egui::ProgressBar::new(
                    f32::from(route.peak.get(channel).copied().unwrap_or_default())
                        / f32::from(u16::MAX),
                ),
            );
        });
        if device_changed {
            changed = true;
            if !route.manual_channel_control && is_leader {
                if let Some((partner, _)) = paired_with {
                    let selected = route.channel_device_ids[channel].clone();
                    route.channel_device_ids[partner].clone_from(&selected);
                }
            }
        }
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
    for (index, route) in routing.inputs.iter_mut().enumerate() {
        if index > 0 {
            ui.separator();
        }
        if !route.connected {
            ui.horizontal(|ui| {
                ui.label(&route.label);
                ui.weak("Disconnected");
            });
            continue;
        }
        if route.source_channels == 1 {
            changed |= draw_compact_input_route(ui, route, devices, capture_channel_labels);
        } else {
            changed |= draw_multichannel_input_route(ui, route, devices, capture_channel_labels);
        }
    }
    changed
}

fn input_stereo_channel_pairs(route: &InputRoute, labels: &[String]) -> Vec<(usize, usize)> {
    if labels.is_empty() {
        return (0..route.source_channels)
            .step_by(2)
            .filter_map(|left| (left + 1 < route.source_channels).then_some((left, left + 1)))
            .collect();
    }
    super::audio_topology::stereo_channel_pairs(labels)
        .into_iter()
        .filter_map(|(left_source, right_source)| {
            let left = route
                .target_channels
                .iter()
                .position(|source| *source == Some(left_source))?;
            let right = route
                .target_channels
                .iter()
                .position(|source| *source == Some(right_source))?;
            Some((left, right))
        })
        .collect()
}

fn sync_input_stereo_channels(route: &mut InputRoute, pairs: &[(usize, usize)]) -> bool {
    let mut changed = false;
    if !route.manual_channel_control {
        for &(left, right) in pairs {
            if route.channel_device_ids[left] != route.channel_device_ids[right] {
                let selected = route.channel_device_ids[left].clone();
                route.channel_device_ids[right].clone_from(&selected);
                changed = true;
            }
        }
    }
    changed
}

fn draw_compact_input_route(
    ui: &mut egui::Ui,
    route: &mut InputRoute,
    devices: &[HostDevice],
    capture_channel_labels: &[String],
) -> bool {
    let mut changed = false;
    let peak = route.peak.first().copied().unwrap_or_default();
    ui.horizontal(|ui| {
        ui.add_sized(
            [120.0, ui.spacing().interact_size.y],
            egui::Label::new(format!(
                "{} ({}):",
                route.label,
                input_channel_label(route, 0, capture_channel_labels)
            ))
                .truncate(),
        );
        changed |= ui
            .checkbox(&mut route.enabled, "Enabled")
            .on_hover_text(
                "The emulated microphone is always present. Enable this to forward samples from the selected host input to it.",
            )
            .changed();
        changed |= draw_device_selector_sized(
            ui,
            (&route.id, "host-input"),
            &mut route.channel_device_ids[0],
            devices,
            105.0,
        );
        let width = ui.available_width().clamp(0.0, 100.0);
        ui.add_sized(
            [width, ui.spacing().interact_size.y],
            egui::ProgressBar::new(f32::from(peak) / f32::from(u16::MAX)),
        );
    });
    changed
}

fn draw_multichannel_input_route(
    ui: &mut egui::Ui,
    route: &mut InputRoute,
    devices: &[HostDevice],
    capture_channel_labels: &[String],
) -> bool {
    let pairs = input_stereo_channel_pairs(route, capture_channel_labels);
    let mut changed = sync_input_stereo_channels(route, &pairs);
    let was_manual = route.manual_channel_control;
    ui.horizontal(|ui| {
        ui.label(&route.label);
        changed |= ui
            .checkbox(&mut route.enabled, "Enabled")
            .on_hover_text(
                "The emulated microphone is always present. Enable this to forward samples from the selected host input to it.",
            )
            .changed();
        if !pairs.is_empty() {
            changed |= ui
                .checkbox(&mut route.manual_channel_control, "Override")
                .on_hover_text(
                    "Allow paired microphone channels to use separate host input devices",
                )
                .changed();
        }
    });
    if was_manual && !route.manual_channel_control {
        changed |= sync_input_stereo_channels(route, &pairs);
    }
    for channel in 0..route.source_channels {
        let paired_with = pairs.iter().find_map(|&(left, right)| {
            if channel == left {
                Some((right, true))
            } else if channel == right {
                Some((left, false))
            } else {
                None
            }
        });
        let is_leader = paired_with.is_some_and(|(_, leader)| leader);
        let is_secondary = paired_with.is_some_and(|(_, leader)| !leader);
        let mut device_changed = false;
        ui.horizontal(|ui| {
            ui.add_sized(
                [170.0, ui.spacing().interact_size.y],
                egui::Label::new(format!(
                    "Channel {} ({}):",
                    channel + 1,
                    input_channel_label(route, channel, capture_channel_labels)
                ))
                .truncate(),
            );
            ui.add_enabled_ui(route.manual_channel_control || !is_secondary, |ui| {
                device_changed = draw_device_selector_sized(
                    ui,
                    (&route.id, "host-input", channel),
                    &mut route.channel_device_ids[channel],
                    devices,
                    AUDIO_DEVICE_SELECTOR_WIDTH,
                );
            });
            let peak = route.peak.get(channel).copied().unwrap_or_default();
            let width = ui.available_width().clamp(0.0, AUDIO_VU_WIDTH);
            ui.add_sized(
                [width, ui.spacing().interact_size.y],
                egui::ProgressBar::new(f32::from(peak) / f32::from(u16::MAX)),
            );
        });
        if device_changed {
            changed = true;
            if !route.manual_channel_control && is_leader {
                if let Some((partner, _)) = paired_with {
                    let selected = route.channel_device_ids[channel].clone();
                    route.channel_device_ids[partner].clone_from(&selected);
                }
            }
        }
    }
    changed
}

fn input_channel_label<'a>(route: &InputRoute, channel: usize, labels: &'a [String]) -> &'a str {
    route
        .target_channels
        .get(channel)
        .copied()
        .flatten()
        .and_then(|index| labels.get(index))
        .map_or("unmapped", String::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use virtualgamepad::AudioChannel;

    #[derive(Default)]
    struct FakeRouter {
        input_blocks: RefCell<VecDeque<Vec<i16>>>,
        output_blocks: RefCell<Vec<Vec<i16>>>,
    }

    impl HostSampleRouter for FakeRouter {
        fn output(&self, _id: &str, samples: &[i16], _channels: usize) -> bool {
            self.output_blocks.borrow_mut().push(samples.to_vec());
            true
        }

        fn input(&self, _id: &str, _channels: usize) -> Option<Vec<i16>> {
            self.input_blocks.borrow_mut().pop_front()
        }
    }

    #[test]
    fn changing_audio_peaks_preserves_the_configured_host_streams() {
        let playback = PcmFormat::new(
            48_000,
            &[AudioChannel::AudibleLeft, AudioChannel::AudibleRight],
        )
        .unwrap();
        let microphone = PcmFormat::new(48_000, &[AudioChannel::Microphone]).unwrap();
        for backend in HostBackend::ALL {
            let mut activity = Activity {
                routing: RoutingState::for_topology(
                    AudioTopology::for_family(ControllerFamily::DualSense),
                    backend,
                ),
                ..Activity::default()
            };
            activity.routing.outputs[0].source_channels = vec![Some(0)];
            activity.routing.inputs[0].enabled = true;
            let config = (activity.routing.configuration(), true, true);
            // A cached sentinel proves configure takes the reuse path without
            // needing a hardware stream or launching a host subprocess.
            let sentinel = Some("synthetic cached stream status".to_owned());
            let mut router = AudioRouter {
                config: Some(config.clone()),
                last_error: sentinel.clone(),
                ..AudioRouter::default()
            };
            let fake_router = FakeRouter::default();
            let mut io = Fake::default();
            for level in [5_i16, 20, 100, 0] {
                assert_eq!(router.configure(&activity.routing, true, true), sentinel);
                fake_router
                    .input_blocks
                    .borrow_mut()
                    .push_back(vec![level; 512]);
                activity
                    .tick(&mut io, &fake_router, Some(&playback), Some(&microphone))
                    .unwrap();
                assert_eq!(activity.routing.outputs[0].peak, [7]);
                assert_eq!(activity.routing.inputs[0].peak, [level.unsigned_abs()]);
                assert_eq!(router.configure(&activity.routing, true, true), sentinel);
                assert_eq!(router.config, Some(config.clone()));
            }
            router.reset();
            router.reset();
            assert!(router.config.is_none());
        }
    }

    #[test]
    fn routing_configuration_retains_device_mapping_and_lifecycle_changes() {
        let topology = AudioTopology::for_family(ControllerFamily::DualSense);
        let routing = RoutingState::for_topology(topology, HostBackend::Alsa);
        let original = routing.configuration();
        let mut changed = routing.clone();
        changed.outputs[0].channel_device_ids[0] = "synthetic-output".into();
        assert_ne!(changed.configuration(), original);
        changed = routing.clone();
        changed.inputs[0].channel_device_ids[0] = "synthetic-input".into();
        assert_ne!(changed.configuration(), original);
        changed = routing.clone();
        changed.inputs[0].enabled = true;
        assert_ne!(changed.configuration(), original);
        changed = routing.clone();
        changed.outputs[0].source_channels[0] = Some(0);
        assert_ne!(changed.configuration(), original);
        changed = routing.clone();
        changed.inputs[0].target_channels[0] = None;
        assert_ne!(changed.configuration(), original);
        changed = routing.clone();
        changed.backend = HostBackend::PipeWire;
        assert_ne!(changed.configuration(), original);
        changed = routing.clone();
        changed.set_jack_connected(true, topology);
        assert_ne!(changed.configuration(), original);
        changed.set_jack_connected(false, topology);
        assert_eq!(changed.configuration(), original);

        let mut disabled = routing;
        disabled.outputs[0].enabled = false;
        let mut router = AudioRouter::default();
        router.configure(&disabled, true, true);
        assert_eq!(router.config, Some((disabled.configuration(), true, true)));
        router.configure(&disabled, false, false);
        assert_eq!(
            router.config,
            Some((disabled.configuration(), false, false))
        );
    }

    #[test]
    fn captured_microphone_blocks_keep_their_length_and_partial_write_suffix() {
        for channels in 1..=2 {
            let channel_labels = [AudioChannel::MicrophoneLeft, AudioChannel::MicrophoneRight];
            let format = PcmFormat::new(48_000, &channel_labels[..channels]).unwrap();
            let mut activity = Activity {
                routing: RoutingState::for_topology(
                    AudioTopology::for_family(ControllerFamily::DualSense),
                    HostBackend::Alsa,
                ),
                ..Activity::default()
            };
            let route = &mut activity.routing.inputs[0];
            route.enabled = true;
            route.source_channels = channels;
            route.target_channels = (0..channels).map(Some).collect();
            let router = FakeRouter::default();
            let mut io = Fake {
                accepted: 13,
                write_channels: channels,
                ..Fake::default()
            };
            for frames in [256, 73, 1] {
                let block = vec![42; frames * channels];
                router.input_blocks.borrow_mut().push_back(block.clone());
                activity
                    .tick(&mut io, &router, None, Some(&format))
                    .unwrap();
                assert_eq!(io.writes.last().unwrap(), &block);
                while !activity.pending.is_empty() {
                    let suffix = activity.pending[activity.accepted_samples..].to_vec();
                    activity
                        .tick(&mut io, &router, None, Some(&format))
                        .unwrap();
                    assert_eq!(io.writes.last().unwrap(), &suffix);
                }
                let writes = io.writes.len();
                activity
                    .tick(&mut io, &router, None, Some(&format))
                    .unwrap();
                assert_eq!(io.writes.len(), writes);
            }
            assert_eq!(activity.microphone_frames, 330);
            assert_eq!(activity.accepted_samples, 0);

            router
                .input_blocks
                .borrow_mut()
                .push_back(vec![42; 256 * channels]);
            activity
                .tick(&mut io, &router, None, Some(&format))
                .unwrap();
            activity.apply(&mut io, Action::FlushMicrophone).unwrap();
            activity.apply(&mut io, Action::FlushMicrophone).unwrap();
            assert!(activity.pending.is_empty());
            assert_eq!(activity.accepted_samples, 0);
        }
    }

    #[test]
    fn test_tone_still_fills_a_complete_block_when_capture_is_shorter() {
        let format = PcmFormat::new(48_000, &[AudioChannel::Microphone]).unwrap();
        let mut activity = Activity {
            routing: RoutingState::for_topology(
                AudioTopology::for_family(ControllerFamily::DualSense),
                HostBackend::Alsa,
            ),
            ..Activity::default()
        };
        activity.routing.inputs[0].enabled = true;
        let router = FakeRouter::default();
        router.input_blocks.borrow_mut().push_back(vec![42; 512]);
        let mut io = Fake {
            accepted: FRAMES,
            ..Fake::default()
        };
        activity.apply(&mut io, Action::Tone(true)).unwrap();
        activity
            .tick(&mut io, &router, None, Some(&format))
            .unwrap();
        assert_eq!(io.writes[0].len(), FRAMES);
        for (frame, sample) in io.writes[0].iter().copied().enumerate() {
            let tone = if (frame * 880 / 48_000) % 2 == 0 {
                1024
            } else {
                -1024
            };
            assert_eq!(sample, tone + if frame < 256 { 42 } else { 0 });
        }
        assert!(activity.pending.is_empty());
    }

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
    fn audio_diagnostic_rows_keep_their_height_as_live_counts_change() {
        let context = egui::Context::default();
        let mut previous_height: Option<f32> = None;
        for frame in 0..6_u64 {
            let mut rect = egui::Rect::NOTHING;
            let rows = [
                ("Status", "Open".to_owned()),
                ("Underrun frames", (frame * 100_000).to_string()),
                ("Playback frames", (frame * 1_000_000).to_string()),
            ];
            let _ = context.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(620.0, 240.0),
                    )),
                    ..Default::default()
                },
                |context| {
                    egui::CentralPanel::default().show(context, |ui| {
                        rect = draw_diagnostic_metric_rows(ui, &rows);
                    });
                },
            );
            assert!(rect.height() > 0.0);
            if let Some(previous_height) = previous_height.filter(|_| frame > 1) {
                assert!((rect.height() - previous_height).abs() < f32::EPSILON);
            }
            previous_height = Some(rect.height());
        }
    }

    #[test]
    fn speaker_and_microphone_cards_fit_with_onboard_and_jack_routes() {
        let context = egui::Context::default();
        let topology = AudioTopology::for_family(ControllerFamily::DualSense);
        let mut routing = RoutingState::for_topology(topology, HostBackend::Alsa);
        routing.set_jack_connected(true, topology);
        assert_eq!(routing.outputs.len(), 2);
        assert_eq!(routing.inputs.len(), 2);
        assert_eq!(routing.inputs[0].source_channels, 2);
        assert_eq!(routing.inputs[0].target_channels, [Some(0), Some(1)]);
        assert_eq!(routing.inputs[1].source_channels, 1);
        assert_eq!(routing.inputs[1].target_channels, [Some(0)]);
        let mut devices = host_devices(HostBackend::Alsa);
        devices[0].label = "System default · USB Audio DAC · Front Left / Right Outputs".into();
        let playback_labels =
            ["Front left", "Front right", "Rear left", "Rear right"].map(str::to_owned);
        let capture_labels = ["Microphone left", "Microphone right"].map(str::to_owned);
        let mut output_rect = egui::Rect::NOTHING;
        let mut battery_rect = egui::Rect::NOTHING;
        let mut microphone_rect = egui::Rect::NOTHING;
        let _ = context.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(500.0, 900.0),
                )),
                ..Default::default()
            },
            |context| {
                egui::CentralPanel::default().show(context, |ui| {
                    output_rect = super::super::wide_card(ui, "Audio", |ui| {
                        draw_output_routes(ui, &mut routing, &devices, &playback_labels);
                    })
                    .response
                    .rect;
                    battery_rect = super::super::input_clusters::card(ui, "Battery", |_| {})
                        .response
                        .rect;
                    microphone_rect = super::super::wide_card_with_heading(
                        ui,
                        |ui| {
                            ui.horizontal(|ui| {
                                ui.strong("Microphone");
                                ui.small_button("!").on_hover_text(
                                    "Microphones are always present on the emulated controller. Host audio is forwarded only when Enabled is selected.",
                                );
                            });
                        },
                        |ui| draw_input_routes(ui, &mut routing, &devices, &capture_labels),
                    )
                    .response
                    .rect;
                });
            },
        );
        assert!(
            output_rect.width() <= 500.0,
            "audio card width: {}",
            output_rect.width()
        );
        assert!(microphone_rect.width() <= 500.0);
        assert!(microphone_rect.top() >= battery_rect.bottom());
        assert_eq!(routing.inputs[0].source_channels, 2);
        assert_eq!(routing.inputs[0].target_channels, [Some(0), Some(1)]);
        assert_eq!(routing.inputs[1].source_channels, 1);
        assert_eq!(routing.inputs[1].target_channels, [Some(0)]);
    }

    #[test]
    fn single_channel_microphone_uses_one_compact_device_row() {
        let context = egui::Context::default();
        let mut routing = RoutingState::for_topology(
            AudioTopology::for_family(ControllerFamily::Xbox360),
            HostBackend::Alsa,
        );
        routing.select_jack_device(
            Some(JackDevice::Headset),
            AudioTopology::for_family(ControllerFamily::Xbox360),
        );
        let devices = host_devices(HostBackend::Alsa);
        let mut row_height = 0.0;
        let _ = context.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(700.0, 200.0),
                )),
                ..Default::default()
            },
            |context| {
                egui::CentralPanel::default().show(context, |ui| {
                    let top = ui.cursor().top();
                    draw_input_routes(ui, &mut routing, &devices, &[]);
                    row_height = ui.cursor().top() - top;
                });
            },
        );
        assert_eq!(routing.inputs.len(), 1);
        assert_eq!(routing.inputs[0].source_channels, 1);
        assert!(
            row_height <= 30.0,
            "single-channel layout used {row_height}px"
        );
    }

    #[test]
    fn disconnected_headset_microphone_keeps_a_visible_status_row() {
        let context = egui::Context::default();
        let mut routing = RoutingState::for_topology(
            AudioTopology::for_family(ControllerFamily::Xbox360),
            HostBackend::Alsa,
        );
        assert_eq!(routing.inputs.len(), 1);
        assert_eq!(routing.inputs[0].label, "Wired headset microphone");
        assert!(!routing.inputs[0].connected);
        let devices = host_devices(HostBackend::Alsa);
        let mut row_height = 0.0;
        let _ = context.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(700.0, 200.0),
                )),
                ..Default::default()
            },
            |context| {
                egui::CentralPanel::default().show(context, |ui| {
                    let top = ui.cursor().top();
                    draw_input_routes(ui, &mut routing, &devices, &[]);
                    row_height = ui.cursor().top() - top;
                });
            },
        );
        assert!(row_height > 0.0);
        assert!(!routing.inputs[0].connected);
    }

    #[test]
    fn active_audio_surface_follows_the_creation_audio_setting() {
        assert!(!controller_audio_enabled(CreationAudio {
            enabled: false,
            ..CreationAudio::default()
        }));
        assert!(controller_audio_enabled(CreationAudio {
            enabled: true,
            ..CreationAudio::default()
        }));
    }

    #[test]
    fn backend_status_explains_the_missing_build_feature_or_target() {
        assert_eq!(
            backend_status(RealizationId::LINUX_UHID_USB, true),
            if cfg!(all(target_os = "linux", feature = "audio-pipewire")) {
                None
            } else {
                Some("Build with --features audio-pipewire")
            }
        );
        assert_eq!(
            backend_status(RealizationId::LINUX_USBIP_USB_AUDIO, true),
            if cfg!(all(target_os = "linux", feature = "audio-usbip")) {
                None
            } else {
                Some("Build with --features audio-usbip")
            }
        );
        assert_eq!(
            backend_status(RealizationId::LINUX_UINPUT, true),
            Some("Audio requires UHID or USB/IP")
        );
        assert_eq!(
            backend_status(RealizationId::LINUX_UHID_USB, false),
            Some("Unsupported for this family")
        );
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
            ["onboard-microphone", "jack-microphone"]
        );
        assert!(!state.inputs[0].enabled);
        assert!(!state.inputs[1].connected);
        assert!(state.inputs[1].label.contains("TRRS"));
        assert_eq!(state.outputs[0].source_channels, [None]);
        assert_eq!(state.outputs[0].channel_device_ids, ["default"]);
        state.set_jack_connected(true, topology);
        assert_eq!(state.outputs.len(), 2);
        assert_eq!(state.outputs[1].source_channels, [Some(0), Some(1)]);
        assert_eq!(state.outputs[1].channel_device_ids, ["default", "default"]);
        assert_eq!(state.inputs.len(), 2);
        assert!(state.inputs[1].label.contains("TRRS"));
        assert!(state.inputs[1].connected);
        assert_eq!(state.inputs[0].id, "onboard-microphone");
        assert_eq!(state.inputs[1].id, "jack-microphone");
        state.inputs[0].channel_device_ids = vec!["built-in-capture".into(); 2];
        state.inputs[1].channel_device_ids = vec!["headset-capture".into()];
        assert_ne!(
            state.inputs[0].channel_device_ids[0],
            state.inputs[1].channel_device_ids[0]
        );
        state.set_jack_device(JackDevice::Microphone, topology);
        assert_eq!(state.outputs.len(), 1);
        assert_eq!(state.inputs.len(), 2);
        state.set_jack_connected(false, topology);
        assert_eq!(state.outputs.len(), 1);
        assert_eq!(state.inputs.len(), 2);
        assert!(!state.inputs[1].connected);
        assert!(state.inputs[1].label.contains("TRRS"));
    }

    #[test]
    fn provider_semantic_channel_mapping_follows_backend_order() {
        let playback = [
            AudioChannel::HapticLeft,
            AudioChannel::AudibleRight,
            AudioChannel::AudibleLeft,
            AudioChannel::HapticRight,
        ];
        assert_eq!(
            semantic_channel_index(&playback, AudioChannel::AudibleRight),
            Some(1)
        );
        assert_eq!(
            semantic_channel_index(&playback, AudioChannel::AudibleLeft),
            Some(2)
        );
        assert_eq!(
            semantic_channel_index(&playback, AudioChannel::Speaker),
            None
        );
    }

    #[test]
    fn selecting_a_jack_device_controls_its_plug_state_and_none_unplugs_it() {
        let topology = AudioTopology::for_family(ControllerFamily::DualSense);
        let mut state = RoutingState::for_topology(topology, HostBackend::Alsa);

        assert!(!state.jack_connected);
        assert_eq!(state.outputs.len(), 1);
        state.select_jack_device(Some(JackDevice::Headset), topology);
        assert!(state.jack_connected);
        assert_eq!(state.outputs.len(), 2);
        assert_eq!(state.inputs.len(), 2);
        assert!(state.inputs[1].connected);

        state.select_jack_device(None, topology);
        assert!(!state.jack_connected);
        assert_eq!(state.outputs.len(), 1);
        assert_eq!(state.inputs.len(), 2);
        assert!(!state.inputs[1].connected);
        assert_eq!(state.inputs[1].label, "TRRS headset microphone");
    }

    #[test]
    fn linked_stereo_shares_a_host_sink_and_manual_channels_can_split_sinks() {
        let topology = AudioTopology::for_family(ControllerFamily::DualSense);
        let mut state = RoutingState::for_topology(topology, HostBackend::Alsa);
        state.set_jack_device(JackDevice::Headset, topology);
        state.set_jack_connected(true, topology);
        let route = &mut state.outputs[1];
        assert_eq!(
            output_device_groups(route),
            [OutputDeviceGroup {
                device_id: "default".into(),
                channels: vec![0, 1],
            }]
        );

        route.manual_channel_control = true;
        route.channel_device_ids = vec!["left-sink".into(), "right-sink".into()];
        assert_eq!(
            output_device_groups(route),
            [
                OutputDeviceGroup {
                    device_id: "left-sink".into(),
                    channels: vec![0],
                },
                OutputDeviceGroup {
                    device_id: "right-sink".into(),
                    channels: vec![1],
                },
            ]
        );
    }

    #[test]
    fn linked_stereo_capture_shares_a_host_source_and_override_splits_sources() {
        let topology = AudioTopology::for_family(ControllerFamily::DualSense);
        let mut state = RoutingState::for_topology(topology, HostBackend::Alsa);
        let route = &mut state.inputs[0];
        let labels = ["MicrophoneLeft", "MicrophoneRight"].map(str::to_owned);
        let pairs = input_stereo_channel_pairs(route, &labels);
        assert_eq!(pairs, [(0, 1)]);
        assert_eq!(
            input_device_groups(route),
            [OutputDeviceGroup {
                device_id: "default".into(),
                channels: vec![0, 1],
            }]
        );

        route.manual_channel_control = true;
        route.channel_device_ids = vec!["left-capture".into(), "right-capture".into()];
        assert_eq!(
            input_device_groups(route),
            [
                OutputDeviceGroup {
                    device_id: "left-capture".into(),
                    channels: vec![0],
                },
                OutputDeviceGroup {
                    device_id: "right-capture".into(),
                    channels: vec![1],
                },
            ]
        );
    }

    #[test]
    fn separate_capture_groups_are_reassembled_in_controller_channel_order() {
        assert_eq!(
            collect_input_channels(2, &[(vec![0], vec![10, 20]), (vec![1], vec![11, 21])]),
            Some(vec![10, 11, 20, 21])
        );
    }

    #[test]
    fn host_sink_channel_selection_preserves_interleaved_frame_order() {
        let samples = [10, 11, 20, 21, 30, 31];
        assert_eq!(select_output_channels(&samples, 2, &[0]), [10, 20, 30]);
        assert_eq!(select_output_channels(&samples, 2, &[1]), [11, 21, 31]);
        assert_eq!(select_output_channels(&samples, 2, &[0, 1]), samples);
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
            enabled: false,
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

        assert!(rect.height() > 120.0, "audio choices were hidden: {rect:?}");
    }

    #[derive(Default)]
    struct Fake {
        reads: usize,
        writes: Vec<Vec<i16>>,
        flushes: Vec<SampleDirection>,
        accepted: usize,
        write_channels: usize,
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
            Ok(self
                .accepted
                .min(samples.len() / self.write_channels.max(1)))
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
