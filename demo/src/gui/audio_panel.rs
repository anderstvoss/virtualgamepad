//! Per-controller audio presentation. Only sample-owned streams can be metered:
//! native-client PCM does not pass through the demo.
use super::audio_lab::View;
use eframe::egui;
use virtualgamepad::{
    AudioAccess, AudioEndpoint, AudioEndpointSelector, RealizationId, SampleDirection,
};

fn realization_level(target: RealizationId) -> &'static str {
    if target == RealizationId::LINUX_UINPUT {
        "Generic"
    } else if target == RealizationId::LINUX_UHID_USB
        || target == RealizationId::LINUX_UHID_BLUETOOTH
    {
        "Native HID"
    } else if target == RealizationId::LINUX_USBIP_USB_AUDIO
        || target == RealizationId::LINUX_DUMMY_HCD_USB_HID
    {
        "Real USB"
    } else {
        "Unclassified"
    }
}

fn direction_label(direction: SampleDirection) -> &'static str {
    if direction == SampleDirection::HostToController {
        "Playback"
    } else if direction == SampleDirection::ControllerToHost {
        "Microphone"
    } else {
        "Other"
    }
}

fn selector_text(selector: &AudioEndpointSelector) -> String {
    if let Some(name) = selector.pipewire_node() {
        format!("PipeWire node: {name}")
    } else if let Some((card, device, subdevice)) = selector.alsa_pcm() {
        format!("ALSA: hw:CARD={card},DEV={device},SUBDEV={subdevice}")
    } else {
        format!("Selector: {selector:?}")
    }
}

fn sample_access(endpoints: &[AudioEndpoint], direction: SampleDirection) -> Option<AudioAccess> {
    endpoints
        .iter()
        .find(|endpoint| endpoint.direction() == direction)
        .map(AudioEndpoint::access)
}

fn peak_fraction(peak: u16) -> f32 {
    f32::from(peak) / 32_768.0
}

fn peak_text(peak: u16) -> String {
    if peak == 0 {
        "Silence (−∞ dBFS)".to_owned()
    } else {
        format!("{:.1} dBFS", 20.0 * peak_fraction(peak).log10())
    }
}

fn draw_meter(ui: &mut egui::Ui, label: &str, access: Option<AudioAccess>, peak: u16) {
    ui.horizontal_wrapped(|ui| {
        ui.label(label);
        match access {
            Some(AudioAccess::Samples) => {
                ui.add(
                    egui::ProgressBar::new(peak_fraction(peak))
                        .desired_width(160.0)
                        .text(peak_text(peak)),
                );
            }
            Some(AudioAccess::NativeClient) => {
                ui.weak("Not metered (native-client-owned PCM)");
            }
            _ => {
                ui.weak("Unavailable");
            }
        }
    });
}

pub(super) fn draw_overview(ui: &mut egui::Ui, target: RealizationId, view: &View) {
    egui::Grid::new("controller_audio_overview")
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label("Realization level");
            ui.strong(realization_level(target));
            ui.end_row();
            ui.label("Realization ID");
            ui.monospace(target.as_str());
            ui.end_row();
        });

    ui.strong("Live sample levels (peak of latest serviced PCM batch)");
    draw_meter(
        ui,
        "Playback",
        sample_access(&view.endpoints, SampleDirection::HostToController),
        view.activity.peak,
    );
    draw_meter(
        ui,
        "Microphone return",
        sample_access(&view.endpoints, SampleDirection::ControllerToHost),
        view.activity.microphone_peak,
    );
    ui.weak("These are demo-observed sample peaks, not a physical-device level or end-to-end measurement.");

    ui.strong("Created audio endpoints");
    if view.endpoints.is_empty() {
        ui.weak("No audio endpoints reported for this creation.");
    }
    for endpoint in &view.endpoints {
        ui.horizontal_wrapped(|ui| {
            ui.strong(format!(
                "{} · {} · {:?}",
                direction_label(endpoint.direction()),
                endpoint.group(),
                endpoint.access()
            ));
        });
        ui.monospace(format!("Host: {}", selector_text(endpoint.host())));
        if let Some(caller) = endpoint.caller() {
            ui.monospace(format!("Caller: {}", selector_text(caller)));
        }
    }
    ui.weak(
        "Endpoint selectors are creation-scoped; do not reuse ALSA card numbers after recreation.",
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn realization_levels_classify_current_audio_and_generic_targets() {
        assert_eq!(realization_level(RealizationId::LINUX_UINPUT), "Generic");
        assert_eq!(
            realization_level(RealizationId::LINUX_UHID_USB),
            "Native HID"
        );
        assert_eq!(
            realization_level(RealizationId::LINUX_UHID_BLUETOOTH),
            "Native HID"
        );
        assert_eq!(
            realization_level(RealizationId::LINUX_USBIP_USB_AUDIO),
            "Real USB"
        );
        assert_eq!(
            realization_level(RealizationId::LINUX_DUMMY_HCD_USB_HID),
            "Real USB"
        );
        assert_eq!(
            realization_level(RealizationId::new("synthetic.other")),
            "Unclassified"
        );
    }

    #[test]
    fn peak_meter_covers_silence_full_scale_and_signed_sample_minimum() {
        assert_eq!(peak_fraction(0), 0.0);
        assert_eq!(peak_fraction(16_384), 0.5);
        assert_eq!(peak_fraction(i16::MIN.unsigned_abs()), 1.0);
        assert_eq!(peak_text(0), "Silence (−∞ dBFS)");
        assert_eq!(peak_text(32_768), "0.0 dBFS");
    }

    #[test]
    fn typed_selectors_do_not_infer_device_association() {
        assert_eq!(
            selector_text(&AudioEndpointSelector::PipeWireNode {
                name: "test.session.output".into(),
            }),
            "PipeWire node: test.session.output"
        );
        assert_eq!(
            selector_text(&AudioEndpointSelector::AlsaPcm {
                card_id: "synthetic".into(),
                device: 1,
                subdevice: 0,
            }),
            "ALSA: hw:CARD=synthetic,DEV=1,SUBDEV=0"
        );
    }

    #[test]
    fn directions_are_labeled_from_typed_values() {
        assert_eq!(
            direction_label(SampleDirection::HostToController),
            "Playback"
        );
        assert_eq!(
            direction_label(SampleDirection::ControllerToHost),
            "Microphone"
        );
    }
}
