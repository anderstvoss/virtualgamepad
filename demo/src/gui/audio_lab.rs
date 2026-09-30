//! Demo-owned bounded PCM work; only the root application API is used.
use eframe::egui;
use virtualgamepad::{
    AudioAccess, AudioDiagnostics, AudioEndpoint, AudioError, AudioExposure, AudioOptions,
    ControllerAudio, ControllerError, CreationOptions, PcmFormat, RealizationId, SampleDirection,
};

#[derive(Clone, Copy)]
pub(super) struct CreationAudio {
    pub enabled: bool,
    pub playback: AudioAccess,
    pub microphone: AudioAccess,
}
impl Default for CreationAudio {
    fn default() -> Self {
        Self {
            enabled: default_creation_target() == RealizationId::LINUX_UHID_USB,
            playback: AudioAccess::Samples,
            microphone: AudioAccess::Samples,
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

pub(super) fn draw_creation(
    ui: &mut egui::Ui,
    config: &mut CreationAudio,
    target: RealizationId,
    has_audio: bool,
) {
    let validation = config.options(target, has_audio);
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.checkbox(&mut config.enabled, "Emulated audio");
            let info = ui.add_sized([18.0, 18.0], egui::Button::new("!"));
            if info.hovered() {
                egui::Tooltip::for_widget(&info).at_pointer().show(|ui| {
                    ui.strong("Audio creation details");
                    ui.label("Audio endpoints are created with the controller. Creation choices are immutable; recreate to change them.");
                    ui.label("PipeWire nodes do not auto-connect. ALSA endpoints require an application to open them.");
                    ui.label("The default Samples owners create no caller-side audio clients and route no physical devices.");
                    if target == RealizationId::LINUX_USBIP_USB_AUDIO {
                        ui.label("USB/IP is opt-in WIP; installed security and recovery acceptance remains open.");
                    }
                    if let Err(error) = &validation {
                        ui.colored_label(egui::Color32::YELLOW, error.to_string());
                    }
                });
            }
            if !has_audio {
                ui.colored_label(egui::Color32::YELLOW, "Unsupported for this family");
            } else if !backend_available(target) {
                ui.colored_label(egui::Color32::YELLOW, "Backend unavailable");
            } else if target == RealizationId::LINUX_USBIP_USB_AUDIO && !config.enabled {
                ui.colored_label(egui::Color32::YELLOW, "USB/IP requires audio");
            }
        });
        ui.add_enabled_ui(config.enabled && has_audio && backend_available(target), |ui| {
            ui.horizontal(|ui| {
                for (label, access) in [
                    ("Playback", &mut config.playback),
                    ("Microphone", &mut config.microphone),
                ] {
                    egui::ComboBox::from_id_salt(label)
                        .selected_text(format!("{label}: {access:?}"))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(access, AudioAccess::Samples, "Samples");
                            ui.selectable_value(access, AudioAccess::NativeClient, "Native client");
                        });
                }
            });
        });
    });
}

#[derive(Clone, Copy, Debug)]
pub(super) enum Action {
    FlushPlayback,
    FlushMicrophone,
    Tone(bool),
}
#[derive(Clone, Debug, Default)]
pub(super) struct Activity {
    pub playback_frames: u64,
    pub microphone_frames: u64,
    pub discontinuities: u64,
    pub first_playback_frame: Option<u64>,
    pub peak: u16,
    pub tone: bool,
    pending: Vec<i16>,
    accepted_samples: usize,
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
        }
    }
    fn tick<I: SampleIo>(
        &mut self,
        io: &mut I,
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
        }
        if self.tone {
            if let Some(format) = microphone {
                let channels = format.channels().len();
                if self.pending.is_empty() {
                    for frame in 0..FRAMES {
                        let position = self.microphone_frames.saturating_add(frame as u64);
                        let sample = if (position.saturating_mul(880)
                            / u64::from(format.sample_rate_hz()))
                            % 2
                            == 0
                        {
                            1024
                        } else {
                            -1024
                        };
                        self.pending.extend(std::iter::repeat_n(sample, channels));
                    }
                }
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
}

#[derive(Clone, Debug)]
pub(super) struct View {
    pub endpoints: Vec<AudioEndpoint>,
    pub health: AudioDiagnostics,
    pub limitation: &'static str,
    pub activity: Activity,
}
pub(super) fn snapshot(audio: &ControllerAudio, activity: &Activity) -> View {
    View {
        endpoints: audio.endpoints().to_vec(),
        health: audio.diagnostics(),
        limitation: audio.limitation(),
        activity: Activity {
            pending: Vec::new(),
            ..activity.clone()
        },
    }
}
pub(super) fn cycle(
    audio: &mut ControllerAudio,
    activity: &mut Activity,
    action: Option<Action>,
) -> Result<(), AudioError> {
    if let Some(action) = action {
        if matches!(action, Action::Tone(true))
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
    activity.tick(audio, playback.as_ref(), microphone.as_ref())
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
        ui.label(format!("Closed: {} · Failed: {} · Underrun frames: {} · Dropped playback: {}",
            view.health.is_closed(), view.health.failed(), view.health.underrun_frames(), view.health.dropped_playback_frames()));
        if let Some(error) = view.health.last_error() { ui.colored_label(egui::Color32::YELLOW, error_message(error)); }
        ui.label(format!("Playback monitor: {} frames, {} discontinuities, peak {} · Latest segment at {:?} · Microphone test: {} accepted frames",
            view.activity.playback_frames, view.activity.discontinuities, view.activity.peak, view.activity.first_playback_frame, view.activity.microphone_frames));
        ui.weak("Samples playback is drained by the monitor. Microphone tone is synthetic and off initially. Native clients own their PCM. No physical audio routing is performed.");
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

#[cfg(test)]
mod tests {
    use super::*;
    use virtualgamepad::AudioChannel;
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
        activity.tick(&mut fake, None, Some(&format)).unwrap();
        activity.tick(&mut fake, None, Some(&format)).unwrap();
        assert_eq!(fake.writes[0].len(), FRAMES);
        assert_eq!(fake.writes[1], fake.writes[0][13..]);
        assert_eq!(activity.microphone_frames, 26);
        activity.apply(&mut fake, Action::FlushPlayback).unwrap();
        assert_eq!(activity.pending.len(), FRAMES);
        activity.apply(&mut fake, Action::Tone(false)).unwrap();
        activity.tick(&mut fake, None, Some(&format)).unwrap();
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
        activity.tick(&mut fake, None, None).unwrap();
        assert_eq!(fake.reads, 0);
        assert!(fake.writes.is_empty());
        fake.fail = true;
        let format = PcmFormat::new(48_000, &[AudioChannel::AudibleLeft]).unwrap();
        let error = activity.tick(&mut fake, Some(&format), None).unwrap_err();
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
            activity.tick(&mut fake, Some(&format), None).unwrap();
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
        assert!(activity.tick(&mut fake, None, Some(&format)).is_err());
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
