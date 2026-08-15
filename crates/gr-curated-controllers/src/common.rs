use crate::CreationOptions;
use gr_controller_contract::{
    CommitError, ControlError, DpadDirection, FaceButton, PreparedRealization,
    TargetAwareControllerDriver, prepare_deployment_realization,
};
use gr_controller_runtime::{ControllerRuntime, FrameSink};
use gr_provider_linux_uhid::LinuxUhidProvider;
use gr_provider_linux_uinput::LinuxUinputProvider;
use gr_realization_api::{
    DeploymentTarget, NativeControllerRealization, NativeDeviceIdentity, NativeHidRealization,
    NativeProviderFactory, NativeProviderSession, ProviderError, ProviderFrame,
    ProviderOpenRequest, ProviderReverseEvent, RawReverseEvent,
};
use std::collections::BTreeMap;

pub(crate) const EV_SYN: u16 = 0;
pub(crate) const EV_KEY: u16 = 1;
pub(crate) const EV_ABS: u16 = 3;
pub(crate) const EV_FF: u16 = 21;
pub(crate) const SYN_REPORT: u16 = 0;

pub(crate) const fn face_index(button: FaceButton) -> usize {
    match button {
        FaceButton::South => 0,
        FaceButton::East => 1,
        FaceButton::West => 2,
        FaceButton::North => 3,
    }
}

pub(crate) const fn dpad_index(direction: DpadDirection) -> usize {
    match direction {
        DpadDirection::Up => 0,
        DpadDirection::Down => 1,
        DpadDirection::Left => 2,
        DpadDirection::Right => 3,
    }
}

pub(crate) struct ProviderSessionSink(Box<dyn NativeProviderSession>);

impl FrameSink for ProviderSessionSink {
    type Frame = ProviderFrame;

    fn send(&mut self, frame: ProviderFrame) -> Result<(), CommitError> {
        self.0.send(frame).map_err(|error| CommitError::Backend {
            reason: error.to_string(),
        })
    }
}

impl ProviderSessionSink {
    pub(crate) fn drain(
        &mut self,
        callback: &mut dyn FnMut(RawReverseEvent),
    ) -> Result<(), ProviderError> {
        let mut events: Vec<ProviderReverseEvent> = Vec::new();
        match self.0.drain_reverse_events(&mut events) {
            Ok(()) => {}
            Err(ProviderError::WouldBlock) => return Ok(()),
            Err(error) => return Err(error),
        }
        for event in events {
            callback(event.event);
        }
        Ok(())
    }

    pub(crate) fn reply(&mut self, frame: ProviderFrame) -> Result<(), ProviderError> {
        self.0.send(frame)
    }
}

pub(crate) fn create<D>(
    driver: D,
    mut realization: NativeControllerRealization,
    options: CreationOptions,
) -> Result<ControllerRuntime<D, ProviderSessionSink>, ProviderError>
where
    D: TargetAwareControllerDriver<Frame = ProviderFrame>,
{
    let prepared: PreparedRealization = prepare_deployment_realization(&driver, options.target)
        .map_err(|error| ProviderError::Unsupported {
            reason: error.to_string(),
        })?;
    if let NativeControllerRealization::Hid(specification) = &mut realization {
        let suffix = format!("session-{}", options.session.0);
        specification.physical_path = format!("{}/{}", specification.physical_path, suffix);
        specification.unique_id = format!("{}-{suffix}", specification.unique_id);
    }
    let request = ProviderOpenRequest {
        session: options.session,
        selection: prepared.selection(),
        requirements: prepared.entry().provider_requirements,
        realization,
    };
    let session: Box<dyn NativeProviderSession> = match options.target {
        DeploymentTarget::Evdev => LinuxUinputProvider.open(request)?,
        DeploymentTarget::Hid => LinuxUhidProvider.open(request)?,
        _ => {
            return Err(ProviderError::Unsupported {
                reason: "unknown deployment target".into(),
            });
        }
    };
    ControllerRuntime::new(driver, ProviderSessionSink(session), prepared).map_err(|error| {
        ProviderError::Open {
            reason: error.to_string(),
        }
    })
}

/// Project-defined HID gamepad report descriptor. It is deliberately a
/// standard HID surface: controller packages own identities and semantics.
pub(crate) const STANDARD_GAMEPAD_DESCRIPTOR: &[u8] = &[
    0x05, 0x01, 0x09, 0x05, 0xa1, 0x01, 0x15, 0x00, 0x25, 0x01, 0x35, 0x00, 0x45, 0x01, 0x75, 0x01,
    0x95, 0x10, 0x05, 0x09, 0x19, 0x01, 0x29, 0x10, 0x81, 0x02, 0x05, 0x01, 0x25, 0x07, 0x46, 0x3b,
    0x01, 0x75, 0x04, 0x95, 0x01, 0x65, 0x14, 0x09, 0x39, 0x81, 0x42, 0x65, 0x00, 0x75, 0x04, 0x95,
    0x01, 0x81, 0x03, 0x15, 0x81, 0x25, 0x7f, 0x75, 0x08, 0x95, 0x06, 0x09, 0x30, 0x09, 0x31, 0x09,
    0x32, 0x09, 0x33, 0x09, 0x34, 0x09, 0x35, 0x81, 0x02, 0xc0,
];

pub(crate) fn hid_realization(
    name: &str,
    vendor_id: u16,
    product_id: u16,
) -> NativeControllerRealization {
    NativeControllerRealization::Hid(NativeHidRealization {
        bus_type: 0x03,
        device_name: name.into(),
        physical_path: "virtualgamepad/uhid".into(),
        unique_id: format!("virtualgamepad-{vendor_id:04x}-{product_id:04x}"),
        identity: NativeDeviceIdentity {
            vendor_id,
            product_id,
            version: 1,
        },
        descriptor: STANDARD_GAMEPAD_DESCRIPTOR.to_vec(),
        numbered_input_reports: false,
        numbered_output_reports: false,
        numbered_feature_reports: false,
        feature_report_responses: BTreeMap::new(),
    })
}

pub(crate) fn hid_gamepad_frame(
    face: [bool; 4],
    dpad: [bool; 4],
    buttons: &[bool],
    axes: [u8; 6],
) -> ProviderFrame {
    let mut bits = 0_u16;
    for (index, pressed) in face.into_iter().chain(buttons.iter().copied()).enumerate() {
        if pressed && index < 16 {
            bits |= 1 << index;
        }
    }
    let hat = match (dpad[0], dpad[1], dpad[2], dpad[3]) {
        (true, false, false, false) => 0,
        (true, false, false, true) => 1,
        (false, false, false, true) => 2,
        (false, true, false, true) => 3,
        (false, true, false, false) => 4,
        (false, true, true, false) => 5,
        (false, false, true, false) => 6,
        (true, false, true, false) => 7,
        _ => 8,
    };
    let mut bytes = Vec::with_capacity(9);
    bytes.extend_from_slice(&bits.to_le_bytes());
    bytes.push(hat);
    bytes.extend_from_slice(&axes);
    ProviderFrame::HidInput {
        report_id: None,
        bytes,
    }
}

pub(crate) fn unavailable(target: gr_realization_api::RealizationTarget) -> ControlError {
    ControlError::UnavailableInRealization {
        selected_target: target,
        available_in: gr_realization_api::RealizationTargetSet::EMPTY,
    }
}
