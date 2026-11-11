//! Linux observations use each creation's public association, never a global first device.
use crate::Backend;
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};
use virtualgamepad::{CreationOptions, DigitalControlUpdate, InputTopology, RealizationId};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Capture(PathBuf);
impl Drop for Capture {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

macro_rules! controllers {
    ($(($variant:ident,$ty:ident,$create:ident,$id:literal $(,$common:ident)?)),+ $(,)?) => {
        pub enum Controller { $($variant(virtualgamepad::$ty),)+ }
        impl Controller {
            fn create(id:&str,target:RealizationId)->Result<Self,String>{
                match id { $($id=>virtualgamepad::$create(CreationOptions::new(target), Default::default()).map(Self::$variant).map_err(|e|e.to_string()),)+ _=>Err("unmapped compiled family".into()) }
            }
            fn physical_paths(&self)->Vec<String> { match self { $(Self::$variant(c)=>c.association().components().iter().filter_map(|c|c.requested_physical_path().map(str::to_owned)).collect(),)+ } }
            fn topology(&self)->&InputTopology { match self { $(Self::$variant(c)=>c.surface()$(.$common())?.input_topology(),)+ } }
            fn digital(&mut self,update:DigitalControlUpdate)->Result<(),String>{match self {$(Self::$variant(c)=>c.set_digital(update).map_err(|e|e.to_string()),)+}}
            fn commit(&mut self)->Result<(),String>{match self {$(Self::$variant(c)=>c.commit().map_err(|e|e.to_string()),)+}}
            fn service(&mut self)->Result<(),String>{match self {$(Self::$variant(c)=>c.service(&mut |_|{}).map_err(|e|e.to_string()),)+}}
            fn close(&mut self){match self {$(Self::$variant(c)=>c.close(),)+}}
        }
        pub const MAPPED_IDS:&[&str]=&[$($id,)+];
    };
}
controllers! {
    (DualSense,DualSenseController,create_dualsense,"virtualgamepad.dualsense",common),
    (DualShock4,DualShock4Controller,create_dualshock4,"virtualgamepad.dualshock4",common),
    (SwitchPro,SwitchProController,create_switch_pro,"virtualgamepad.switch-pro",common),
    (Xbox360,Xbox360Controller,create_xbox360,"virtualgamepad.xbox360",common),
    (Wii,WiiRemoteController,create_wii_remote,"virtualgamepad.wii-remote"),
    (Steam,SteamController2026Controller,create_steam_controller_2026,"virtualgamepad.steam-controller-2026"),
    (XboxSeries,XboxSeriesController,create_xbox_series,"virtualgamepad.xbox-series"),
}

pub struct Linux {
    pub sdl: bool,
    pub root: PathBuf,
}
pub struct Session {
    controller: Controller,
    physical_paths: Vec<String>,
    nodes: BTreeSet<PathBuf>,
    consumers: Vec<File>,
    service: crate::PeerService,
}

fn matching_nodes(physical_paths: &[String]) -> BTreeSet<PathBuf> {
    let mut result = BTreeSet::new();
    for (class, prefix) in [("input", "event"), ("hidraw", "hidraw")] {
        let Ok(entries) = fs::read_dir(format!("/sys/class/{class}")) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !name.starts_with(prefix) {
                continue;
            }
            let device = entry.path().join("device");
            let phys = fs::read_to_string(device.join("phys"))
                .ok()
                .map(|s| s.trim().to_owned())
                .or_else(|| {
                    fs::read_to_string(device.join("uevent"))
                        .ok()?
                        .lines()
                        .find_map(|s| s.strip_prefix("HID_PHYS=").map(str::to_owned))
                });
            if phys.is_some_and(|phys| physical_paths.contains(&phys)) {
                result.insert(PathBuf::from(if class == "input" {
                    format!("/dev/input/{name}")
                } else {
                    format!("/dev/{name}")
                }));
            }
        }
    }
    result
}
fn open_consumer(node: &Path, wait_for_udev: bool) -> std::io::Result<File> {
    let deadline = Instant::now() + Duration::from_secs(2);
    retry_consumer_open(
        || {
            OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NONBLOCK)
                .open(node)
        },
        || {
            if !wait_for_udev || Instant::now() >= deadline {
                return false;
            }
            // Sysfs enumeration may precede the session's evdev ACL.
            thread::sleep(Duration::from_millis(1));
            true
        },
    )
}

fn retry_consumer_open<T>(
    mut open: impl FnMut() -> std::io::Result<T>,
    mut wait: impl FnMut() -> bool,
) -> std::io::Result<T> {
    loop {
        match open() {
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::NotFound
                ) && wait() => {}
            result => return result,
        }
    }
}

fn evdev_capabilities(node: &Path) -> Option<Value> {
    if !node.starts_with("/dev/input") {
        return None;
    }
    let device = Path::new("/sys/class/input")
        .join(node.file_name()?)
        .join("device");
    let read = |name: &str| {
        fs::read_to_string(device.join(name))
            .ok()
            .map(|s| s.trim().to_owned())
    };
    Some(json!({
        "name": read("name"),
        "bus": read("id/bustype"),
        "vendor": read("id/vendor"),
        "product": read("id/product"),
        "properties": read("properties"),
        "keys": read("capabilities/key"),
        "absolute_axes": read("capabilities/abs"),
        "misc": read("capabilities/msc"),
        "force_feedback": read("capabilities/ff"),
    }))
}
fn bound_driver(node: &Path) -> Option<String> {
    let name = node.file_name()?;
    let class = if node.starts_with("/dev/input") {
        "input"
    } else {
        "hidraw"
    };
    let path = fs::canonicalize(
        Path::new("/sys/class")
            .join(class)
            .join(name)
            .join("device"),
    )
    .ok()?;
    path.ancestors().find_map(|p| {
        fs::read_link(p.join("driver"))
            .ok()?
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
    })
}
fn steam_sample(
    sample: i16,
    target: virtualgamepad::RealizationId,
) -> virtualgamepad::SteamController2026State {
    use virtualgamepad::SteamController2026Button as B;
    const BUTTONS: [B; 22] = [
        B::A,
        B::B,
        B::X,
        B::Y,
        B::Qam,
        B::RightStick,
        B::Menu,
        B::R4,
        B::R5,
        B::RightShoulder,
        B::Down,
        B::Right,
        B::Left,
        B::Up,
        B::View,
        B::LeftStick,
        B::Steam,
        B::L4,
        B::L5,
        B::LeftShoulder,
        B::RightTrigger,
        B::LeftTrigger,
    ];
    let mut state = virtualgamepad::SteamController2026State::default();
    let wired = target == virtualgamepad::RealizationId::LINUX_UHID_USB;
    if wired {
        state.motion.timestamp_us = u32::from(u16::from_ne_bytes(sample.to_ne_bytes())) * 10_000;
        state.motion.accelerometer = [sample % 250 * 100, 500, 1000];
        state.motion.gyroscope = [sample % 250 * 30, -250, 100];
    }
    let value = if sample % 20 < 10 { 10000 } else { -10000 };
    state.sticks = [[value, -value], [-value, value]];
    state.triggers = [value.max(0), (-value).max(0)];
    for (index, button) in BUTTONS.into_iter().enumerate() {
        state.set_button(
            button,
            usize::try_from(sample / 6).ok() == Some(index) && sample % 6 < 4,
        );
    }
    for (index, pad) in state.pads.iter_mut().enumerate() {
        pad.touch = (sample % 20 < 10) == (index == 0);
        pad.position = [value + sample % 10 * 100, -value + sample % 10 * 60];
        pad.pressure = if wired && pad.touch { 1000 } else { 0 };
        pad.click = pad.touch && sample % 5 < 2;
    }
    if wired {
        state.stick_touch = [value > 0, value < 0];
        state.grip_touch = [value < 0, value > 0];
    }
    state
}
fn drive_steam_sample(
    steam: &mut virtualgamepad::SteamController2026Controller,
    sample: i16,
) -> Result<(), String> {
    steam
        .set_inputs(steam_sample(sample, steam.surface().target()))
        .map_err(|error| error.to_string())?;
    steam.commit().map_err(|error| error.to_string())
}
fn xbox_series_sample(sample: i16) -> virtualgamepad::XboxSeriesState {
    use virtualgamepad::XboxSeriesButton as B;
    const BUTTONS: [B; 16] = [
        B::A,
        B::B,
        B::X,
        B::Y,
        B::LeftShoulder,
        B::RightShoulder,
        B::View,
        B::Menu,
        B::Guide,
        B::Share,
        B::LeftStickPress,
        B::RightStickPress,
        B::DpadUp,
        B::DpadDown,
        B::DpadLeft,
        B::DpadRight,
    ];
    let mut state = virtualgamepad::XboxSeriesState::default();
    if let Some(button) = usize::try_from(sample / 4)
        .ok()
        .and_then(|index| BUTTONS.get(index))
    {
        state.set_button(*button, sample % 4 < 3);
    }
    let axis = match sample {
        8..=12 => i16::MIN,
        16..=20 => i16::MAX,
        _ => 0,
    };
    state.sticks = [[axis; 2]; 2];
    if (24..=28).contains(&sample) {
        state
            .set_triggers(1023, 1023)
            .expect("Xbox Series trigger maximum");
    }
    state
}

#[derive(Default)]
struct ProbeScript {
    next_ms: Option<u64>,
    sample: i16,
}
impl ProbeScript {
    fn advance(&mut self, now_ms: u64, ready: bool) -> Option<i16> {
        if !ready {
            return None;
        }
        let next = *self.next_ms.get_or_insert(now_ms + 10);
        if now_ms < next {
            return None;
        }
        self.next_ms = Some(now_ms + 10);
        self.sample += 1;
        Some(self.sample)
    }
}
fn probe_ready(text: &str) -> bool {
    text.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .any(|record| record["record_type"] == "mapping_ready")
}
#[derive(Default)]
struct XboxProbeFeedback {
    evdev: Option<virtualgamepad::RumblePlayback>,
    bluetooth: Option<virtualgamepad::XboxSeriesBluetoothRumble>,
    reports: Vec<Value>,
}
fn independent_trigger_reports(reports: &[Value]) -> bool {
    [
        [15, 25, 0, 25, 50, 255, 0, 235],
        [15, 0, 50, 25, 50, 255, 0, 235],
    ]
    .iter()
    .all(|bytes| {
        reports
            .iter()
            .any(|report| report["report_id"] == 3 && report["bytes"] == json!(bytes))
    })
}
fn series_nodes_ready(nodes: &BTreeSet<PathBuf>, bluetooth: bool) -> bool {
    probe_node(nodes, false).is_some() && (!bluetooth || probe_node(nodes, true).is_some())
}
fn probe_node(nodes: &BTreeSet<PathBuf>, hidraw: bool) -> Option<&PathBuf> {
    nodes
        .iter()
        .find(|node| node.starts_with("/dev/input") != hidraw)
}
fn xbox_hidapi() -> bool {
    std::env::var("VIRTUALGAMEPAD_XBOX_HIDAPI").as_deref() == Ok("1")
}

fn wait_child(
    child: &mut Child,
    controller: &mut Controller,
    peers: &mut [&mut Session],
    capture: &Path,
) -> Result<XboxProbeFeedback, String> {
    let start = Instant::now();
    let deadline = start + Duration::from_secs(30);
    let mut script = ProbeScript::default();
    let mut ready = false;
    let mut feedback = XboxProbeFeedback::default();
    loop {
        for peer in peers.iter_mut() {
            peer.service.service(|| peer.controller.service());
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                return if status.success() {
                    Ok(feedback)
                } else {
                    Err(format!("SDL probe exited {status}"))
                };
            }
            Ok(None) => {}
            Err(error) => return Err(error.to_string()),
        }
        ready = ready || fs::read_to_string(capture).is_ok_and(|text| probe_ready(&text));
        let sample = script.advance(
            u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX),
            ready,
        );
        let service = if let Controller::XboxSeries(series) = &mut *controller {
            series
                .service(&mut |event| {
                    if let virtualgamepad::XboxSeriesOutputEvent::HidOutput { report_id, bytes } =
                        event
                    {
                        feedback
                            .reports
                            .push(json!({"report_id":report_id,"bytes":bytes}));
                    }
                })
                .map_err(|error| error.to_string())
        } else {
            controller.service()
        };
        let result = service.and_then(|()| match (&mut *controller, sample) {
            (Controller::Steam(steam), Some(sample)) => drive_steam_sample(steam, sample),
            (Controller::XboxSeries(series), Some(sample)) if sample <= 64 => {
                series
                    .set_inputs(xbox_series_sample(sample))
                    .map_err(|error| error.to_string())?;
                series.commit().map_err(|error| error.to_string())
            }
            _ => Ok(()),
        });
        if let Err(error) = result {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
        if let Controller::XboxSeries(series) = controller {
            if let Ok(outputs) = series.host_outputs() {
                let playback = outputs.evdev_rumble.filter(|playback| {
                    playback.repetitions > 0 && (playback.strong != 0 || playback.weak != 0)
                });
                feedback.evdev = playback.or(feedback.evdev);
                let bluetooth = outputs
                    .bluetooth_rumble
                    .filter(|playback| playback.strong != 0 || playback.weak != 0);
                feedback.bluetooth = bluetooth.or(feedback.bluetooth);
            }
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("SDL probe timeout".into());
        }
        thread::sleep(Duration::from_millis(1));
    }
}
impl Backend for Linux {
    type Session = Session;

    fn observes_live_virtual_devices(&self) -> bool {
        true
    }

    fn create(&mut self, family: &str, realization: &str) -> Result<Session, String> {
        let target = gr_curated_controllers::registry::FAMILIES
            .iter()
            .find(|f| f.id.as_str() == family)
            .and_then(|f| {
                (f.manifest)()
                    .entries()
                    .iter()
                    .find(|e| e.target.as_str() == realization)
            })
            .ok_or("undeclared realization")?
            .target;
        let controller = Controller::create(family, target)?;
        let physical_paths = controller.physical_paths();
        Ok(Session {
            controller,
            physical_paths,
            nodes: BTreeSet::new(),
            consumers: Vec::new(),
            service: crate::PeerService::default(),
        })
    }
    fn observe(
        &mut self,
        s: &mut Session,
        stage: &str,
        peers: &mut [&mut Session],
    ) -> Result<Value, String> {
        match stage {
            "node_present"|"node_gone"=>{
                let deadline=Instant::now()+Duration::from_secs(2);
                loop {
                    for peer in peers.iter_mut() { peer.service.service(|| peer.controller.service()); }
                    let nodes=matching_nodes(&s.physical_paths);
                    let steam_hid = matches!(&s.controller, Controller::Steam(c)
                        if c.surface().target() == RealizationId::LINUX_UHID_USB);
                    let series_ready = match &s.controller {
                        Controller::XboxSeries(c) => series_nodes_ready(&nodes, c.surface().target() == RealizationId::LINUX_UHID_BLUETOOTH),
                        _ => true,
                    };
                    if stage=="node_present" && !nodes.is_empty() && series_ready
                        && (!steam_hid || nodes.iter().any(|p| !p.starts_with("/dev/input"))) {
                        s.nodes=nodes;
                        return Ok(json!({"poll_interval_ns":1_000_000,"devices":s.nodes.iter().map(|p| {
                            let driver = bound_driver(p);
                            let mut device = json!({"path":p,"bound_driver":driver,"driver_reason":if driver.is_none(){Some("no bound driver observed")}else{None}});
                            if let Some(capabilities) = matches!(&s.controller, Controller::Steam(_) | Controller::XboxSeries(_))
                                .then(|| evdev_capabilities(p)).flatten() {
                                device["evdev_capabilities"] = capabilities;
                            }
                            device
                        }).collect::<Vec<_>>() }));
                    }
                    if stage=="node_gone" && nodes.is_empty(){return Ok(json!({"poll_interval_ns":1_000_000}));}
                    if Instant::now()>=deadline{return Err(format!("{stage} not observed within two seconds"));}
                    if stage=="node_present"{s.controller.service()?;}
                    thread::sleep(Duration::from_millis(1));
                }
            }
            "consumer_open"=>{
                if s.nodes.is_empty(){return Err("no owned node observed".into());}
                let series = matches!(&s.controller, Controller::XboxSeries(_));
                let series_bt = matches!(&s.controller, Controller::XboxSeries(c)
                    if c.surface().target() == RealizationId::LINUX_UHID_BLUETOOTH);
                for node in s.nodes.iter().filter(|node| !series_bt || node.starts_with("/dev/input")) {
                    s.consumers.push(open_consumer(node, series).map_err(|e|e.to_string())?);
                }
                if s.consumers.is_empty() { return Err("no owned consumer node opened".into()); }
                let consumer = if series_bt { "harness evdev" } else { "harness evdev/hidraw" };
                Ok(json!({"consumer":consumer,"nodes_opened":s.consumers.len()}))
            }
            "first_neutral"=>Err("no validated consumer neutral-report oracle for this family; API commit is not consumer delivery".into()),
            "sdl_ready"=>{
                if !self.sdl{return Err("SDL probe not requested".into());}
                let steam_hid = matches!(&s.controller, Controller::Steam(c)
                    if c.surface().target() == RealizationId::LINUX_UHID_USB);
                let series = matches!(&s.controller, Controller::XboxSeries(_));
                let series_hidapi = matches!(&s.controller, Controller::XboxSeries(c)
                    if c.surface().target() == RealizationId::LINUX_UHID_BLUETOOTH) && xbox_hidapi();
                let node=probe_node(&s.nodes, steam_hid || series_hidapi).ok_or("no owned node for requested SDL backend")?;
                let path = std::env::temp_dir().join(format!("virtualgamepad-acceptance-{}-{}.json", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
                let file = OpenOptions::new().write(true).create_new(true).open(&path).map_err(|e|e.to_string())?;
                let capture = Capture(path);
                if series_hidapi && bound_driver(node).as_deref() != Some("microsoft") {
                    return Err("Xbox HIDAPI acceptance requires stock microsoft binding".into());
                }
                // Discovery precedes udev access; wait for this owned raw node only.
                if series_hidapi { drop(open_consumer(node, true).map_err(|e| e.to_string())?); }
                let mut command = Command::new("bash");
                command.arg(self.root.join("scripts/run-sdl3-gamepad-probe.sh"))
                    .arg(node).arg(if steam_hid || series { "1800" } else { "100" });
                if series {
                    command.arg(if series_hidapi { "--gamepad-trigger-rumble-script" } else { "--gamepad-rumble-script" }).arg("--reopen");
                    command.env("SDL_JOYSTICK_HIDAPI", if series_hidapi { "1" } else { "0" });
                }
                let mut child = command.stdout(Stdio::from(file)).stderr(Stdio::null())
                    .spawn().map_err(|e|e.to_string())?;
                let status = wait_child(&mut child, &mut s.controller, peers, &capture.0);
                let text = fs::read_to_string(&capture.0).map_err(|e|e.to_string())?;
                let record = text.lines().filter_map(|line|serde_json::from_str::<Value>(line).ok()).find(|r|r["consumer"]=="SDL").ok_or("SDL probe emitted no observation")?;
                if steam_hid && status.is_err() {
                    return Err(format!("SDL probe failed: {}", record["error"]));
                }
                let feedback = status?;
                if record["observations"]["opened"]["value"] != true { return Err("SDL did not observe this exact device opened".into()); }
                let mut result = json!({"consumer":"SDL","timestamp_scope":"probe completion upper bound; discovery may occur earlier","observation":record});
                if series {
                    if series_hidapi && !independent_trigger_reports(&feedback.reports) {
                        return Err("independent Xbox trigger command reports not received".into());
                    }
                    result["hid_output_reports"] = json!(feedback.reports);
                    result["bound_driver_after"] = json!(bound_driver(node));
                    result["trigger_submissions"] = json!(text.lines().filter_map(|line| serde_json::from_str::<Value>(line).ok()).filter(|r| r["record_type"] == "trigger_rumble_submission").collect::<Vec<_>>());
                    if series_hidapi && bound_driver(node).as_deref() != Some("microsoft") {
                        return Err("Xbox binding changed during HIDAPI acceptance".into());
                    }
                    result["accepted_evdev_rumble"] = feedback.evdev.map_or(Value::Null, |playback| json!({
                        "strong":playback.strong, "weak":playback.weak, "repetitions":playback.repetitions
                    }));
                    result["accepted_bluetooth_rumble"] = feedback.bluetooth.map_or(Value::Null, |effect| json!({
                        "mask":effect.motor_mask, "strong":effect.strong, "weak":effect.weak,
                        "left_trigger":effect.left_trigger, "right_trigger":effect.right_trigger
                    }));
                }
                Ok(result)
            }
            _=>Err("unknown stage".into()),
        }
    }
    fn drive(&mut self, s: &mut Session) -> Result<(), String> {
        let buttons: Vec<_> = s
            .controller
            .topology()
            .face_button_clusters()
            .iter()
            .flat_map(|cluster| cluster.buttons().iter().map(|b| b.button))
            .collect();
        s.controller.commit()?;
        for button in buttons {
            for pressed in [true, false] {
                s.controller
                    .digital(DigitalControlUpdate::FaceButton { button, pressed })?;
                s.controller.commit()?;
                s.controller.service()?;
            }
        }
        Ok(())
    }
    fn close(&mut self, s: &mut Session) {
        s.consumers.clear();
        s.controller.close();
    }
    fn take_service_errors(&mut self, session: &mut Session) -> Vec<String> {
        session.service.take_errors()
    }
    fn resources(&mut self) -> Value {
        fn count(path: &str) -> Option<usize> {
            fs::read_dir(path).ok().map(Iterator::count)
        }
        json!({"fds":count("/proc/self/fd"),"threads":count("/proc/self/task"),"input_nodes":count("/sys/class/input"),"hidraw_nodes":count("/sys/class/hidraw")})
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn trigger_acceptance_requires_both_exact_independent_reports() {
        use serde_json::json;
        let left = json!({"report_id":3,"bytes":[15,25,0,25,50,255,0,235]});
        let right = json!({"report_id":3,"bytes":[15,0,50,25,50,255,0,235]});
        assert!(!super::independent_trigger_reports(&[]));
        assert!(!super::independent_trigger_reports(std::slice::from_ref(
            &left
        )));
        assert!(super::independent_trigger_reports(&[
            left.clone(),
            right.clone()
        ]));
        for invalid in [
            json!({"report_id":1,"bytes":[15,0,50,25,50,255,0,235]}),
            json!({"report_id":3,"bytes":[15,25,50,25,50,255,0,235]}),
            json!({"report_id":3,"bytes":[15,0,50]}),
        ] {
            assert!(!super::independent_trigger_reports(&[
                left.clone(),
                invalid
            ]));
        }
    }

    #[test]
    fn series_enumeration_waits_for_input_after_hidraw_and_both_for_bluetooth() {
        use std::{collections::BTreeSet, path::PathBuf};
        let input = PathBuf::from("/dev/input/event999");
        let raw = PathBuf::from("/dev/hidraw999");
        for (nodes, uinput, bluetooth) in [
            (BTreeSet::new(), false, false),
            (BTreeSet::from([raw.clone()]), false, false),
            (BTreeSet::from([input.clone()]), true, false),
            (BTreeSet::from([input, raw]), true, true),
        ] {
            assert_eq!(super::series_nodes_ready(&nodes, false), uinput);
            assert_eq!(super::series_nodes_ready(&nodes, true), bluetooth);
        }
    }

    #[test]
    fn requested_probe_backend_never_falls_back_to_the_other_node_type() {
        use std::{collections::BTreeSet, path::PathBuf};
        let input = PathBuf::from("/dev/input/event999");
        let raw = PathBuf::from("/dev/hidraw999");
        for nodes in [
            BTreeSet::new(),
            BTreeSet::from([input.clone()]),
            BTreeSet::from([raw.clone()]),
            BTreeSet::from([input.clone(), raw.clone()]),
        ] {
            for (hidraw, expected) in [(false, &input), (true, &raw)] {
                assert_eq!(super::probe_node(&nodes, hidraw), nodes.get(expected));
            }
        }
    }

    #[test]
    fn steam_samples_use_the_selected_realizations_input_capabilities() {
        use virtualgamepad::{RealizationId, SteamMotionSample};
        for sample in 1..=138 {
            let reduced = super::steam_sample(sample, RealizationId::LINUX_UINPUT);
            let wired = super::steam_sample(sample, RealizationId::LINUX_UHID_USB);
            assert_eq!(reduced.motion, SteamMotionSample::default());
            assert_eq!(reduced.stick_touch, [false; 2]);
            assert_eq!(reduced.grip_touch, [false; 2]);
            assert_eq!(reduced.sticks, wired.sticks);
            assert_eq!(reduced.triggers, wired.triggers);
            assert_ne!(wired.motion, SteamMotionSample::default());
            assert!(wired.stick_touch.iter().any(|contact| *contact));
            assert!(wired.grip_touch.iter().any(|contact| *contact));
            for side in 0..2 {
                assert_eq!(reduced.pads[side].pressure, 0);
                assert_eq!(
                    wired.pads[side].pressure,
                    if wired.pads[side].touch { 1000 } else { 0 }
                );
                assert_eq!(reduced.pads[side].touch, wired.pads[side].touch);
                assert_eq!(reduced.pads[side].click, wired.pads[side].click);
                assert_eq!(reduced.pads[side].position, wired.pads[side].position);
            }
        }
    }

    #[test]
    fn steam_controls_wait_for_consumer_neutral_after_arbitrary_startup_delay() {
        use virtualgamepad::SteamController2026Button as B;
        assert!(
            super::steam_sample(1, virtualgamepad::RealizationId::LINUX_UHID_USB).pressed(B::A)
        );
        assert!(
            !super::steam_sample(4, virtualgamepad::RealizationId::LINUX_UHID_USB).pressed(B::A)
        );
        assert!(
            super::steam_sample(127, virtualgamepad::RealizationId::LINUX_UHID_USB)
                .pressed(B::LeftTrigger)
        );
        for pad in 0..2 {
            assert_ne!(
                super::steam_sample(1, virtualgamepad::RealizationId::LINUX_UHID_USB).pads[pad]
                    .position,
                super::steam_sample(2, virtualgamepad::RealizationId::LINUX_UHID_USB).pads[pad]
                    .position
            );
        }

        for delay in [0, 350, 1500, 5000] {
            let mut script = super::ProbeScript::default();
            assert_eq!(script.advance(delay, false), None);
            assert_eq!(script.advance(delay, true), None);
            for sample in 1..=138 {
                assert_eq!(
                    script.advance(delay + sample * 10, true),
                    Some(i16::try_from(sample).unwrap())
                );
            }
        }
        assert!(!super::probe_ready("{\"consumer\":\"SDL\"}\n{partial"));
        assert!(super::probe_ready("{\"record_type\":\"mapping_ready\"}\n"));
    }
    #[test]
    fn series_consumer_open_recovers_from_delayed_node_and_acl() {
        use std::io::ErrorKind::{NotFound, PermissionDenied, Unsupported};
        let mut results = [Err(NotFound.into()), Err(PermissionDenied.into()), Ok(7)].into_iter();
        let mut waits = 0;
        let opened = super::retry_consumer_open(
            || results.next().unwrap(),
            || {
                waits += 1;
                true
            },
        )
        .unwrap();
        assert_eq!((opened, waits), (7, 2));
        assert_eq!(
            super::retry_consumer_open::<()>(|| Err(PermissionDenied.into()), || false,)
                .unwrap_err()
                .kind(),
            PermissionDenied
        );
        assert_eq!(
            super::retry_consumer_open::<()>(
                || Err(Unsupported.into()),
                || panic!("terminal open error"),
            )
            .unwrap_err()
            .kind(),
            Unsupported
        );
    }

    #[test]
    fn xbox_series_sweep_includes_share_guide_axes_and_neutral() {
        use virtualgamepad::{XboxSeriesButton as B, XboxSeriesState, XboxSeriesTrigger};
        assert!(super::xbox_series_sample(32).pressed(B::Guide));
        assert!(!super::xbox_series_sample(32).pressed(B::Share));
        assert!(super::xbox_series_sample(36).pressed(B::Share));
        assert!(!super::xbox_series_sample(36).pressed(B::Guide));
        assert_eq!(super::xbox_series_sample(8).sticks, [[i16::MIN; 2]; 2]);
        assert_eq!(super::xbox_series_sample(16).sticks, [[i16::MAX; 2]; 2]);
        assert_eq!(
            super::xbox_series_sample(24)
                .triggers()
                .map(XboxSeriesTrigger::raw),
            [1023; 2]
        );
        assert_eq!(super::xbox_series_sample(64), XboxSeriesState::default());
    }

    #[test]
    fn every_registry_family_has_a_public_constructor_adapter() {
        let mut mapped = super::MAPPED_IDS.to_vec();
        mapped.sort_unstable();
        let mut registry: Vec<_> = gr_curated_controllers::registry::FAMILIES
            .iter()
            .map(|f| f.id.as_str())
            .collect();
        registry.sort_unstable();
        assert_eq!(mapped, registry);
    }
}
