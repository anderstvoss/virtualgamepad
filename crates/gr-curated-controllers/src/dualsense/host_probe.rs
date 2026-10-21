//! Private test instrumentation: no new production realization is advertised.
use super::*;
use std::{
    fs,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

fn devices(prefix: &str) -> Vec<PathBuf> {
    fs::read_dir("/sys/bus/hid/devices")
        .expect("HID inventory")
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let properties = fs::read_to_string(path.join("uevent")).ok()?;
            properties
                .lines()
                .any(|line| {
                    line == format!("HID_PHYS={prefix}")
                        || line.starts_with(&format!("HID_PHYS={prefix}/"))
                })
                .then_some(path)
        })
        .collect()
}

// BUS_VIRTUAL (0x06) comparison retired: selected UHID USB creation owns
// BUS_USB. Historical measurements are unchanged; future reruns verify HID_ID.
const PROBE_BUSES: &[u16] = &[0x03];

fn verify_observed_bus(properties: &str, expected: u16) -> Result<u16, String> {
    let field = properties
        .lines()
        .find_map(|line| line.strip_prefix("HID_ID="))
        .and_then(|id| id.split(':').next())
        .ok_or_else(|| "missing kernel HID_ID".to_owned())?;
    let observed =
        u16::from_str_radix(field, 16).map_err(|_| "invalid kernel HID_ID bus".to_owned())?;
    if observed != expected {
        return Err(format!(
            "kernel bus {observed:#04x} differs from probe bus {expected:#04x}"
        ));
    }
    Ok(observed)
}

#[test]
fn probe_buses_match_selected_realization_and_verify_kernel_identity() {
    assert_eq!(PROBE_BUSES, [0x03]);
    assert_eq!(
        verify_observed_bus("HID_ID=0003:0000054C:00000CE6\n", 3),
        Ok(3)
    );
    for actual in ["0005", "0006"] {
        assert!(verify_observed_bus(&format!("HID_ID={actual}:0000054C:00000CE6\n"), 3).is_err());
    }
    for malformed in ["", "HID_NAME=synthetic", "HID_ID=", "HID_ID=zzzz:0000:0000"] {
        assert!(verify_observed_bus(malformed, 3).is_err());
    }
}

#[test]
#[ignore = "controlled BUS_USB experiment; isolated prepared Linux host only"]
fn controlled_bus_startup_probe() {
    // Three sequential USB repetitions; no driver override or consumer launch.
    for repeat in 0..3 {
        for &bus_type in PROBE_BUSES {
            let options = CreationOptions {
                target: RealizationTarget::LINUX_UHID_USB,
                session: RealizationSessionId(0x4250),
            };
            let mut realization = hid_realization(options.session);
            let NativeControllerRealization::Uhid(specification) = &mut realization else {
                unreachable!()
            };
            let prefix = format!("virtualgamepad/probe/{}", std::process::id());
            specification.physical_path.clone_from(&prefix);
            specification.unique_id = format!("vg-probe-{}", std::process::id());
            assert!(
                devices(&prefix).is_empty(),
                "ambiguous existing probe device"
            );
            let runtime = common::create(DualSenseDefinition, realization, options)
                .expect("open instrumented personality");
            let mut controller = DualSenseController(runtime);
            controller.commit().expect("initial neutral input");
            let start = Instant::now();
            let mut step = 0_i16;
            let mut failure = None;
            while start.elapsed() < Duration::from_secs(5) {
                // Identical caller semantic script/cadence in baseline and rewrite.
                let result = controller
                    .set_motion(MotionSample {
                        accelerometer: [step, 0, 8192],
                        gyroscope: [0, step, 0],
                    })
                    .map_err(|error| error.to_string())
                    .and_then(|()| controller.commit().map_err(|error| error.to_string()))
                    .and_then(|()| {
                        controller
                            .poll_output(&mut |_| {})
                            .map_err(|error| error.to_string())
                    });
                if let Err(error) = result {
                    failure = Some(error);
                    break;
                }
                step = (step + 1) % 1000;
                thread::sleep(Duration::from_millis(4));
            }
            let found = devices(&prefix);
            let observations: Vec<_> = found
                .iter()
                .map(|path| {
                    let driver = fs::read_link(path.join("driver")).ok().and_then(|link| {
                        link.file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                    });
                    (
                        driver,
                        path.join("input").is_dir(),
                        path.join("hidraw").is_dir(),
                        fs::read_to_string(path.join("uevent"))
                            .map_err(|error| error.to_string())
                            .and_then(|properties| verify_observed_bus(&properties, bus_type)),
                    )
                })
                .collect();
            controller.close();
            controller.close();
            let cleanup = Instant::now();
            while !devices(&prefix).is_empty() && cleanup.elapsed() < Duration::from_secs(2) {
                thread::sleep(Duration::from_millis(5));
            }
            let removed = devices(&prefix).is_empty();
            eprintln!(
                "BUS_PROBE repeat={repeat} bus={bus_type:#04x} observations={observations:?} service_error={failure:?} removed={removed}"
            );
            assert!(removed, "probe device survived close");
            assert!(failure.is_none(), "service failed: {failure:?}");
            assert_eq!(found.len(), 1, "probe must observe exactly its own device");
            // Check after terminal cleanup so even a mislabeled/absent HID_ID
            // cannot leave the probe endpoint open on an assertion failure.
            for (_, _, _, observed_bus) in observations {
                assert_eq!(
                    observed_bus,
                    Ok(bus_type),
                    "probe bus must match kernel HID_ID"
                );
            }
        }
    }
}
