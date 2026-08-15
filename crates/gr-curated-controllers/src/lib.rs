#![forbid(unsafe_code)]
#![allow(clippy::missing_errors_doc)]
//! Compiled, controller-native implementations for the curated public API.
//!
//! Controller modules deliberately share transport helpers, not controller
//! state. Numeric values are native to their controller family.

mod common;
pub mod dualsense;
pub mod dualshock4;
pub mod generic_gamepad;
pub mod switch_pro;
pub mod xbox360;

use gr_controller_contract::ControlError;
use gr_realization_api::{DeploymentTarget, RealizationSessionId};

/// Options for ordinary Linux controller creation.
#[derive(Debug, Clone, Copy)]
pub struct CreationOptions {
    pub target: DeploymentTarget,
    pub session: RealizationSessionId,
}

/// Battery percentage shared by every curated controller family.
///
/// Battery exposure is live state rather than a creation option: callers can
/// model changing between an externally powered controller and a wireless
/// controller without disrupting an active provider session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatteryLevel(u8);
impl BatteryLevel {
    #[must_use]
    pub const fn percent(self) -> u8 {
        self.0
    }

    pub fn new(percent: u8) -> Result<Self, ControlError> {
        if percent > 100 {
            return Err(ControlError::ValueOutOfRange {
                control: "battery level",
                value: u32::from(percent),
                maximum: 100,
            });
        }
        Ok(Self(percent))
    }
}

/// Semantic battery state shared by all curated controller families.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatteryState {
    exposed: bool,
    level: BatteryLevel,
}
impl Default for BatteryState {
    fn default() -> Self {
        Self {
            exposed: false,
            level: BatteryLevel(100),
        }
    }
}
impl BatteryState {
    #[must_use]
    pub const fn is_exposed(self) -> bool {
        self.exposed
    }

    #[must_use]
    pub const fn level(self) -> BatteryLevel {
        self.level
    }

    pub(crate) fn set_exposed(&mut self, exposed: bool) {
        self.exposed = exposed;
    }

    pub(crate) fn set_level(&mut self, level: BatteryLevel) {
        self.level = level;
    }
}

pub use dualsense::{
    DualSenseAxis, DualSenseControl, DualSenseController, DualSenseFeature, DualSenseHidOutput,
    DualSenseOutputEvent, DualSenseState, DualSenseSurface, DualSenseTouchContact,
    DualSenseTrigger, DualSenseUsbOptions, MotionSample, TouchSlot, create_dualsense,
    create_dualsense_usb,
};
pub use dualshock4::{
    DualShock4Axis, DualShock4Control, DualShock4Controller, DualShock4HidOutput,
    DualShock4MotionSample, DualShock4OutputEvent, DualShock4State, DualShock4Surface,
    DualShock4TouchContact, DualShock4TouchSlot, DualShock4Trigger, DualShock4UsbOptions,
    create_dualshock4, create_dualshock4_usb,
};
pub use generic_gamepad::{
    GenericGamepadAxis, GenericGamepadControl, GenericGamepadController, GenericGamepadOutputEvent,
    GenericGamepadState, GenericGamepadSurface, GenericGamepadTrigger, create_generic_gamepad,
};
pub use switch_pro::{
    SwitchProAxis, SwitchProControl, SwitchProController, SwitchProMotionSample,
    SwitchProOutputEvent, SwitchProState, SwitchProSurface, SwitchProUsbOptions, create_switch_pro,
    create_switch_pro_usb,
};
pub use xbox360::{
    Xbox360Axis, Xbox360Control, Xbox360Controller, Xbox360OutputEvent, Xbox360State,
    Xbox360Surface, Xbox360Trigger, create_xbox360,
};

#[cfg(test)]
mod battery_tests {
    use super::*;

    #[test]
    fn battery_level_accepts_the_full_percentage_domain_only() {
        assert_eq!(BatteryLevel::new(0).expect("empty battery").percent(), 0);
        assert_eq!(BatteryLevel::new(100).expect("full battery").percent(), 100);
        assert!(BatteryLevel::new(101).is_err());
    }

    #[test]
    fn battery_state_defaults_to_hidden_and_full() {
        let battery = BatteryState::default();
        assert!(!battery.is_exposed());
        assert_eq!(battery.level().percent(), 100);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use gr_controller_contract::{DigitalControlUpdate, FaceButton};
    use gr_realization_api::{DeploymentTarget, RealizationSessionId};
    use std::{
        fs,
        io::Read,
        os::unix::fs::OpenOptionsExt,
        path::PathBuf,
        thread,
        time::{Duration, Instant},
    };

    fn input_node_exists(name: &str) -> bool {
        let Ok(entries) = fs::read_dir("/sys/class/input") else {
            return false;
        };
        entries.flatten().any(|entry| {
            fs::read_to_string(entry.path().join("name"))
                .is_ok_and(|observed| observed.trim() == name)
        })
    }

    fn input_node_count_containing(fragment: &str) -> usize {
        let Ok(entries) = fs::read_dir("/sys/class/input") else {
            return 0;
        };
        entries
            .flatten()
            .filter(|entry| {
                fs::read_to_string(entry.path().join("name"))
                    .is_ok_and(|observed| observed.contains(fragment))
            })
            .count()
    }

    fn input_node_exists_containing(fragment: &str) -> bool {
        input_node_count_containing(fragment) > 0
    }

    fn input_event_path_containing(fragment: &str, physical_path: &str) -> Option<PathBuf> {
        let entries = fs::read_dir("/sys/class/input").ok()?;
        entries.flatten().find_map(|entry| {
            let event = entry.file_name();
            if !event.to_string_lossy().starts_with("event") {
                return None;
            }
            let name = fs::read_to_string(entry.path().join("device/name")).ok()?;
            let physical = fs::read_to_string(entry.path().join("device/phys")).ok()?;
            if name.contains(fragment) && physical.trim() == physical_path {
                Some(PathBuf::from("/dev/input").join(entry.file_name()))
            } else {
                None
            }
        })
    }

    fn read_input_events(file: &mut fs::File) -> Vec<(u16, u16, i32)> {
        let mut events = Vec::new();
        let mut bytes = [0_u8; 24]; // Linux `struct input_event` on supported 64-bit hosts.
        loop {
            match file.read(&mut bytes) {
                Ok(0) => break,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                Ok(24) => events.push((
                    u16::from_ne_bytes([bytes[16], bytes[17]]),
                    u16::from_ne_bytes([bytes[18], bytes[19]]),
                    i32::from_ne_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]),
                )),
                Ok(count) => panic!("truncated Linux input event ({count} bytes)"),
                Err(error) => panic!("read Linux input event: {error}"),
            }
        }
        events
    }

    fn virtual_dualsense_hidraw_path(session: RealizationSessionId) -> Option<PathBuf> {
        let expected_physical_path = format!(
            "HID_PHYS=virtualgamepad/uhid/dualsense/session-{}",
            session.0
        );
        let entries = fs::read_dir("/sys/class/hidraw").ok()?;
        for entry in entries.flatten() {
            let Ok(device) = fs::canonicalize(entry.path().join("device")) else {
                continue;
            };
            if !device
                .to_string_lossy()
                .contains("/devices/virtual/misc/uhid/")
            {
                continue;
            }
            let Ok(uevent) = fs::read_to_string(device.join("uevent")) else {
                continue;
            };
            if uevent.contains("HID_ID=0003:0000054C:00000CE6")
                && uevent.contains(&expected_physical_path)
            {
                return Some(PathBuf::from("/dev").join(entry.file_name()));
            }
        }
        None
    }

    fn read_hid_reports(file: &mut fs::File) -> Vec<Vec<u8>> {
        let mut reports = Vec::new();
        let mut bytes = [0_u8; 64];
        loop {
            match file.read(&mut bytes) {
                Ok(0) => break,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                Ok(64) => reports.push(bytes.to_vec()),
                Ok(count) => panic!("truncated DualSense HID report ({count} bytes)"),
                Err(error) => panic!("read DualSense HID report: {error}"),
            }
        }
        reports
    }

    fn poll_dualsense_for(
        controller: &mut DualSenseController,
        duration: Duration,
        mut after_poll: impl FnMut(),
    ) {
        let deadline = Instant::now() + duration;
        while Instant::now() < deadline {
            controller
                .poll_output(&mut |_| {})
                .expect("UHID output polling must remain live");
            after_poll();
            thread::sleep(Duration::from_millis(50));
        }
    }

    fn poll_for(duration: Duration, mut poll: impl FnMut()) {
        let deadline = Instant::now() + duration;
        while Instant::now() < deadline {
            poll();
            thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    #[ignore = "requires ordinary-user /dev/uinput access"]
    fn all_evdev_controllers_create_commit_changed_state_and_close() {
        let options = |session| CreationOptions {
            target: DeploymentTarget::Evdev,
            session: RealizationSessionId(session),
        };

        let mut generic = create_generic_gamepad(options(101)).expect("generic creation");
        generic
            .set_digital(DigitalControlUpdate::FaceButton {
                button: FaceButton::South,
                pressed: true,
            })
            .expect("generic update");
        generic.commit().expect("generic changed commit");
        generic.close();

        let mut xbox = create_xbox360(options(102)).expect("xbox creation");
        xbox.set_native(Xbox360Control::A, true)
            .expect("xbox update");
        xbox.commit().expect("xbox changed commit");
        xbox.close();

        let mut dualsense = create_dualsense(options(103)).expect("DualSense creation");
        dualsense
            .set_touch(
                TouchSlot::First,
                Some(DualSenseTouchContact::new(0, 960, 470).expect("native contact")),
            )
            .expect("DualSense touch update");
        dualsense.commit().expect("DualSense changed commit");
        dualsense.close();

        let mut dualshock4 = create_dualshock4(options(204)).expect("DualShock 4 creation");
        dualshock4
            .set_digital(DigitalControlUpdate::FaceButton {
                button: FaceButton::South,
                pressed: true,
            })
            .expect("DualShock 4 update");
        dualshock4.commit().expect("DualShock 4 changed commit");
        dualshock4.close();

        let mut switch_pro = create_switch_pro(options(205)).expect("Switch Pro creation");
        switch_pro
            .set_digital(DigitalControlUpdate::FaceButton {
                button: FaceButton::South,
                pressed: true,
            })
            .expect("Switch Pro update");
        switch_pro.commit().expect("Switch Pro changed commit");
        switch_pro.close();
    }

    #[test]
    #[ignore = "requires pre-provisioned /dev/uhid access"]
    fn all_hid_controllers_create_commit_changed_state_and_close() {
        let options = |session| CreationOptions {
            target: DeploymentTarget::Hid,
            session: RealizationSessionId(session),
        };

        let mut generic = create_generic_gamepad(options(201)).expect("generic creation");
        generic
            .set_digital(DigitalControlUpdate::FaceButton {
                button: FaceButton::South,
                pressed: true,
            })
            .expect("generic update");
        generic.commit().expect("generic changed commit");
        generic.close();

        let mut xbox = create_xbox360(options(202)).expect("xbox creation");
        xbox.set_native(Xbox360Control::A, true)
            .expect("xbox update");
        xbox.commit().expect("xbox changed commit");
        xbox.close();

        let mut dualsense = create_dualsense(options(203)).expect("DualSense creation");
        dualsense
            .set_digital(DigitalControlUpdate::FaceButton {
                button: FaceButton::South,
                pressed: true,
            })
            .expect("DualSense update");
        dualsense.commit().expect("DualSense changed commit");
        dualsense.close();

        let mut dualshock4 = create_dualshock4(options(204)).expect("DualShock 4 creation");
        dualshock4
            .set_touch(
                DualShock4TouchSlot::First,
                Some(DualShock4TouchContact::new(1, 100, 100).expect("touch")),
            )
            .expect("DualShock 4 touch update");
        dualshock4.commit().expect("DualShock 4 changed commit");
        dualshock4.close();

        let mut switch_pro = create_switch_pro(options(205)).expect("Switch Pro creation");
        switch_pro
            .set_digital(DigitalControlUpdate::FaceButton {
                button: FaceButton::South,
                pressed: true,
            })
            .expect("Switch Pro update");
        switch_pro.commit().expect("Switch Pro changed commit");
        switch_pro.close();
    }

    #[test]
    #[ignore = "requires pre-provisioned /dev/uhid access"]
    fn multiple_hid_controllers_remain_open_concurrently() {
        let options = |session| CreationOptions {
            target: DeploymentTarget::Hid,
            session: RealizationSessionId(session),
        };
        let mut first = create_generic_gamepad(options(301)).expect("first HID controller");
        let mut second = create_generic_gamepad(options(302)).expect("second HID controller");
        first
            .set_digital(DigitalControlUpdate::FaceButton {
                button: FaceButton::South,
                pressed: true,
            })
            .expect("first update");
        second
            .set_digital(DigitalControlUpdate::FaceButton {
                button: FaceButton::East,
                pressed: true,
            })
            .expect("second update");
        first.commit().expect("first commit");
        second.commit().expect("second commit");
        first.close();
        second.close();
    }

    #[test]
    #[ignore = "requires /dev/uhid and a Linux input subsystem"]
    fn dualsense_hid_materializes_and_survives_the_host_probe_interval() {
        let initial_nodes = input_node_count_containing("DualSense");
        let mut controller = create_dualsense(CreationOptions {
            target: DeploymentTarget::Hid,
            session: RealizationSessionId(401),
        })
        .expect("DualSense UHID creation");
        assert!(
            !controller.is_dirty(),
            "DualSense creation must flush the initial full input report before Steam probes it"
        );
        let mut appeared = false;
        poll_dualsense_for(&mut controller, Duration::from_secs(3), || {
            appeared |= input_node_count_containing("DualSense") > initial_nodes;
        });
        let diagnostics = controller.provider_diagnostics();
        assert!(
            diagnostics.reverse_events_drained >= 3,
            "Linux did not complete the required DualSense feature probes: {diagnostics:?}"
        );
        assert!(appeared, "UHID device never materialized an input node");
        assert!(
            input_node_exists_containing("Virtual DualSense Motion Sensors"),
            "a valid calibration feature must let hid-playstation create the motion device"
        );

        controller
            .set_digital(DigitalControlUpdate::FaceButton {
                button: FaceButton::South,
                pressed: true,
            })
            .expect("post-probe state update");
        controller
            .set_motion(MotionSample {
                gyroscope: [1_000, -1_000, 500],
                accelerometer: [8_192, 0, -8_192],
            })
            .expect("post-probe motion update");
        controller.commit().expect("post-probe input report");
        poll_dualsense_for(&mut controller, Duration::from_secs(3), || {});
        assert!(
            input_node_count_containing("DualSense") > initial_nodes,
            "DualSense input node disappeared during the host probe interval"
        );
        assert!(
            input_node_exists_containing("Virtual DualSense Motion Sensors"),
            "motion sensor node disappeared after a non-zero DualSense motion report"
        );
        controller.close();
    }

    #[test]
    #[ignore = "requires /dev/uinput and a Linux input subsystem"]
    fn dualsense_evdev_materializes_and_survives_the_host_probe_interval() {
        let mut controller = create_dualsense(CreationOptions {
            target: DeploymentTarget::Evdev,
            session: RealizationSessionId(402),
        })
        .expect("DualSense evdev creation");
        controller.commit().expect("initial evdev report");
        assert!(
            input_node_exists("Virtual DualSense"),
            "evdev device did not materialize an input node"
        );

        thread::sleep(Duration::from_secs(3));

        controller
            .set_digital(DigitalControlUpdate::FaceButton {
                button: FaceButton::South,
                pressed: true,
            })
            .expect("post-probe state update");
        controller.commit().expect("post-probe evdev report");
        assert!(
            input_node_exists("Virtual DualSense"),
            "evdev input node disappeared during the host probe interval"
        );
        controller.close();
    }

    #[test]
    #[ignore = "requires /dev/uhid plus read access to the generated hidraw node"]
    fn dualsense_hid_delivers_steam_facing_motion_reports() {
        let session = RealizationSessionId(403);
        let mut controller = create_dualsense(CreationOptions {
            target: DeploymentTarget::Hid,
            session,
        })
        .expect("DualSense UHID creation");
        assert!(
            !controller.is_dirty(),
            "DualSense creation must flush the initial full input report before Steam probes it"
        );
        poll_dualsense_for(&mut controller, Duration::from_secs(3), || {});
        let hidraw = virtual_dualsense_hidraw_path(session)
            .expect("Linux did not create the virtual DualSense hidraw node");
        let mut hidraw = fs::OpenOptions::new()
            .read(true)
            .custom_flags(0o4_000) // O_NONBLOCK; opening never waits for an event.
            .open(&hidraw)
            .unwrap_or_else(|error| panic!("Steam cannot open {}: {error}", hidraw.display()));
        let _ = read_hid_reports(&mut hidraw); // Ignore the initial neutral report.

        let motion = MotionSample {
            gyroscope: [1_000, -2_000, 3_000],
            accelerometer: [-4_000, 5_000, -6_000],
        };
        let deadline = Instant::now() + Duration::from_millis(500);
        let mut reports = Vec::new();
        while Instant::now() < deadline {
            controller.set_motion(motion).expect("motion update");
            controller.commit().expect("motion commit");
            controller
                .poll_output(&mut |_| {})
                .expect("Steam probe polling must remain live during motion");
            reports.extend(read_hid_reports(&mut hidraw));
            thread::sleep(Duration::from_millis(4));
        }
        let motion_timestamps = reports
            .iter()
            .filter(|report| {
                report[0] == 0x01
                    // OpenPuck / hid-playstation wire axes: X, Z, -Y.
                    && report[16..22] == [0xe8, 0x03, 0xb8, 0x0b, 0xd0, 0x07]
                    && report[22..28] == [0x60, 0xf0, 0x88, 0x13, 0x90, 0xe8]
            })
            .map(|report| u32::from_le_bytes(report[28..32].try_into().expect("timestamp")))
            .collect::<Vec<_>>();
        assert!(
            motion_timestamps.len() >= 20,
            "Steam-facing HID stream stopped during sustained motion: {reports:?}"
        );
        assert!(
            motion_timestamps
                .windows(2)
                .all(|timestamps| timestamps[0] != timestamps[1]),
            "sustained motion reports reused sensor timestamps: {motion_timestamps:?}"
        );
        assert!(
            virtual_dualsense_hidraw_path(session).is_some()
                && input_node_exists_containing("Virtual DualSense Motion Sensors"),
            "DualSense ceased to be detectable during sustained motion"
        );
        controller.close();
    }

    #[test]
    #[ignore = "requires /dev/uhid and the Linux PlayStation HID driver"]
    fn dualshock4_hid_materializes_a_timed_motion_and_touch_device() {
        let mut controller = create_dualshock4(CreationOptions {
            target: DeploymentTarget::Hid,
            session: RealizationSessionId(404),
        })
        .expect("DualShock 4 UHID creation");
        controller
            .set_touch(
                DualShock4TouchSlot::First,
                Some(DualShock4TouchContact::new(1, 960, 471).expect("native touch")),
            )
            .expect("touch update");
        poll_for(Duration::from_secs(1), || {
            controller
                .set_motion(DualShock4MotionSample {
                    accelerometer: [1_000, 2_000, 3_000],
                    gyroscope: [4_000, 5_000, 6_000],
                })
                .expect("motion update");
            controller.commit().expect("motion commit");
            controller.poll_output(&mut |_| {}).expect("output poll");
        });
        assert!(
            input_node_exists_containing("Wireless Controller Motion Sensors")
                && input_node_exists("Wireless Controller Touchpad"),
            "DS4 did not materialize its motion and touch input devices"
        );
        controller.close();
    }

    #[test]
    #[ignore = "requires /dev/uhid, /dev/input read access, and the Linux Nintendo HID driver"]
    fn switch_pro_host_handshake_enables_timed_imu_streaming() {
        let session = RealizationSessionId(405);
        let mut controller = create_switch_pro(CreationOptions {
            target: DeploymentTarget::Hid,
            session,
        })
        .expect("Switch Pro UHID creation");
        let counter_before = controller.state().motion_report_counter();
        poll_for(Duration::from_secs(3), || {
            controller.poll_output(&mut |_| {}).expect("output poll");
            controller.refresh_motion().expect("motion refresh");
        });
        assert!(
            controller.state().stream_enabled(),
            "host never completed the Switch Pro 0x30 report-mode handshake"
        );
        let physical_path = format!("virtualgamepad/uhid/switch-pro/session-{}", session.0);
        let imu_path = input_event_path_containing("Pro Controller (IMU)", &physical_path)
            .expect("hid-nintendo did not create the virtual Switch Pro IMU event device");
        let mut imu = match fs::OpenOptions::new()
            .read(true)
            .custom_flags(0o4_000)
            .open(&imu_path)
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                eprintln!(
                    "skipping Switch IMU event assertion: no read access to {}",
                    imu_path.display()
                );
                controller.close();
                return;
            }
            Err(error) => panic!("cannot open {}: {error}", imu_path.display()),
        };
        let _ = read_input_events(&mut imu);
        let motion = SwitchProMotionSample {
            accelerometer: [0; 3],
            gyroscope: [1_000, -2_000, 3_000],
        };
        poll_for(Duration::from_millis(100), || {
            controller.set_motion(motion).expect("motion update");
            controller.refresh_motion().expect("motion refresh");
        });
        let imu_events = read_input_events(&mut imu);
        assert!(
            imu_events.iter().any(|(event_type, code, value)| {
                *event_type == 3 && matches!(*code, 3..=5) && *value != 0
            }),
            "Switch Pro IMU device received no non-zero gyro event: {imu_events:?}"
        );
        assert_ne!(
            controller.state().motion_report_counter(),
            counter_before,
            "Switch Pro report counter did not advance while streaming"
        );
        controller.close();
    }
}
