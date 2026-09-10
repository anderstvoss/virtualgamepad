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

#[test]
#[ignore = "controlled BUS_USB/BUS_VIRTUAL experiment; isolated prepared Linux host only"]
fn controlled_bus_startup_probe() {
    // Three sequential repetitions per bus; no driver override or consumer launch.
    for repeat in 0..3 {
        for bus_type in [0x03, 0x06] {
            let options = CreationOptions {
                target: RealizationTarget::Uhid,
                session: RealizationSessionId(0x4250),
            };
            let mut realization = hid_realization(options.session);
            let NativeControllerRealization::Uhid(specification) = &mut realization else {
                unreachable!()
            };
            specification.bus_type = bus_type;
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
        }
    }
}
