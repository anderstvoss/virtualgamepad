//! Throwaway host probe for Steam UHID selection; never a production controller.
use gr_provider_linux_uhid::LinuxUhidProvider;
use gr_realization_api::{
    ControllerId, NativeControllerRealization, NativeDeviceIdentity, NativeHidRealization,
    NativeProviderFactory, NativeProviderSession, ProviderError, ProviderFrame,
    ProviderOpenRequest, ProviderRequirements, RawReverseEvent, RealizationId,
    RealizationSelection, RealizationSessionId,
};
use std::{
    error::Error,
    fs,
    fs::OpenOptions,
    io::{self, Read},
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use virtualgamepad::{CreationOptions, create_dualsense};

#[derive(Clone, Copy)]
enum Case {
    Ibex,
    Puck,
    DualSense,
}

impl Case {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "ibex" => Some(Self::Ibex),
            "puck" => Some(Self::Puck),
            "dualsense" => Some(Self::DualSense),
            _ => None,
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Ibex => "ibex-1302",
            Self::Puck => "puck-single-1304",
            Self::DualSense => "dualsense-control",
        }
    }

    const fn product(self) -> u16 {
        match self {
            Self::Ibex => 0x1302,
            Self::Puck => 0x1304,
            Self::DualSense => unreachable!(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct HidNode {
    driver: String,
    hidraw: Vec<PathBuf>,
    events: Vec<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let Some(case_name) = args.next() else {
        return Err("usage: cargo run --example steam_uhid_probe --features experimental -- ibex|puck|dualsense [seconds]".into());
    };
    let case = Case::parse(&case_name).ok_or("case must be ibex, puck, or dualsense")?;
    let seconds = args
        .next()
        .map_or(Ok(45_u64), |value| value.parse::<u64>())?;
    if !(5..=300).contains(&seconds) {
        return Err("duration must be between 5 and 300 seconds".into());
    }
    match case {
        Case::Ibex | Case::Puck => run_valve_case(case, seconds),
        Case::DualSense => run_dualsense_control(seconds),
    }
}

fn run_valve_case(case: Case, seconds: u64) -> Result<(), Box<dyn Error>> {
    let process = std::process::id();
    let family = match case {
        Case::Ibex => "ibex",
        Case::Puck => "puck",
        Case::DualSense => unreachable!(),
    };
    let physical_path = format!("virtualgamepad/uhid/steam-probe/{family}/p{process:x}-i0");
    let unique_id = format!("virtualgamepad-steam-probe-{family}-p{process:x}");
    let identity = NativeDeviceIdentity {
        vendor_id: 0x28de,
        product_id: case.product(),
        version: 0x0100,
    };
    let specification = NativeHidRealization {
        target: RealizationId::LINUX_UHID_USB,
        bus_type: 0x03,
        device_name: format!("Steam Controller 2026 {} probe", case.name()),
        physical_path: physical_path.clone(),
        unique_id,
        identity,
        descriptor: valve_descriptor(matches!(case, Case::Puck)),
        numbered_input_reports: true,
        numbered_output_reports: true,
        numbered_feature_reports: true,
    };
    let request = ProviderOpenRequest {
        session: RealizationSessionId(u64::from(process)),
        selection: RealizationSelection {
            controller: ControllerId::new("steam-controller-2026-probe"),
            target: RealizationId::LINUX_UHID_USB,
        },
        requirements: ProviderRequirements::default(),
        realization: NativeControllerRealization::Uhid(specification),
    };
    let provider = LinuxUhidProvider::for_target(RealizationId::LINUX_UHID_USB)?;
    let mut session = provider.open(request)?;
    let result = run_input_and_observe(&mut *session, case, &physical_path, seconds);
    let close = session.close();
    let removed = verify_removed(&physical_path);
    result?;
    close?;
    removed?;
    Ok(())
}

fn valve_descriptor(puck: bool) -> Vec<u8> {
    let top_level_usage = if puck { 0x02 } else { 0x01 };
    vec![
        0x06,
        0x00,
        0xff, // Vendor page FF00.
        0x09,
        top_level_usage, // IBEX usage 1; the Puck single-node rejection uses usage 2.
        0xa1,
        0x01, // Application collection.
        0x85,
        0x42, // Input report 42, 53 payload bytes.
        0x09,
        0x42,
        0x15,
        0x00,
        0x26,
        0xff,
        0x00,
        0x75,
        0x08,
        0x95,
        0x35,
        0x81,
        0x02,
        0x85,
        0x01, // Feature/output command report used by the wired kernel path.
        0x09,
        0x01,
        0x75,
        0x08,
        0x95,
        0x03,
        0x91,
        0x02,
        0x09,
        0x01,
        0x95,
        0x40,
        0xb1,
        0x02,
        0xc0,
    ]
}

fn run_input_and_observe(
    session: &mut dyn NativeProviderSession,
    case: Case,
    physical_path: &str,
    seconds: u64,
) -> Result<(), Box<dyn Error>> {
    let neutral = fixture("steam-c6-neutral-42.hex")?;
    send_fixture(session, &neutral)?;
    let start = Instant::now();
    let deadline = start + Duration::from_secs(seconds);
    let mut metrics = Metrics::default();
    let mut next_input = start;
    let mut submitted = 1_u64;
    let mut next_inventory = start;
    let mut last_inventory = None;
    let mut hidapi_probed = false;
    let mut sdl_probed = false;
    let mut sdl_child = None;
    while Instant::now() < deadline {
        drain_events(session, &mut metrics)?;
        if Instant::now() >= next_input {
            send_fixture(session, &neutral)?;
            submitted += 1;
            next_input = Instant::now() + Duration::from_millis(10);
        }
        if Instant::now() >= next_inventory {
            let nodes = find_hid_nodes(physical_path)?;
            if !nodes.is_empty() && last_inventory.as_ref() != Some(&nodes) {
                print_inventory(case, &nodes);
                last_inventory = Some(nodes.clone());
            }
            if sdl_child.is_none()
                && !sdl_probed
                && let Some(raw) = nodes.iter().flat_map(|node| &node.hidraw).next()
            {
                let access = exact_node_access(raw, case)?;
                if access
                    && !hidapi_probed
                    && std::env::var_os("VIRTUALGAMEPAD_HIDAPI_PROBE").is_some()
                {
                    run_hidapi_probe(raw, case)?;
                    hidapi_probed = true;
                }
                if access && std::env::var_os("VIRTUALGAMEPAD_SDL_PROBE").is_some() {
                    sdl_child = Some(spawn_sdl_probe(raw)?);
                }
            }
            next_inventory = Instant::now() + Duration::from_secs(1);
        }
        poll_sdl_probe(&mut sdl_child, case, &mut sdl_probed)?;
        thread::sleep(Duration::from_millis(2));
    }
    stop_sdl_probe(&mut sdl_child)?;
    drain_events(session, &mut metrics)?;
    println!(
        "{{\"record_type\":\"input_submission\",\"count\":{submitted},\"consumer_delivery\":null,\"reason\":\"submission alone does not prove consumer delivery\"}}"
    );
    send_fixture(session, &neutral)?;
    println!(
        "{{\"record_type\":\"probe_summary\",\"case\":\"{}\",\"output_reports\":{},\"feature_gets\":{},\"feature_sets\":{},\"lizard_disable_seen\":{}}}",
        case.name(),
        metrics.output_reports,
        metrics.feature_gets,
        metrics.feature_sets,
        metrics.lizard_disable_seen
    );
    println!(
        "{{\"record_type\":\"request_accounting\",\"requests\":{},\"replies\":{},\"pending\":{},\"opens\":{},\"closes\":{}}}",
        metrics.requests,
        metrics.replies,
        metrics.requests - metrics.replies,
        metrics.opens,
        metrics.closes
    );
    session.close()?;
    Ok(())
}

#[derive(Default)]
struct Metrics {
    requests: u64,
    replies: u64,
    opens: u64,
    closes: u64,
    output_reports: u64,
    feature_gets: u64,
    feature_sets: u64,
    lizard_disable_seen: bool,
    last_feature_command: Vec<u8>,
}

fn drain_events(
    session: &mut dyn NativeProviderSession,
    metrics: &mut Metrics,
) -> Result<(), ProviderError> {
    loop {
        let mut events = Vec::new();
        match session.drain_reverse_events(&mut events) {
            Ok(()) => {}
            Err(ProviderError::WouldBlock) => return Ok(()),
            Err(error) => return Err(error),
        }
        for event in events {
            match event.event {
                RawReverseEvent::HidOutput { report_id, bytes } => {
                    metrics.output_reports += 1;
                    if report_id == Some(1) {
                        metrics.lizard_disable_seen |= is_kernel_lizard_disable(report_id, &bytes);
                        metrics.last_feature_command = bytes;
                    }
                }
                RawReverseEvent::HidGetReportRequest {
                    request_id,
                    report_id,
                    report_type,
                } => {
                    metrics.feature_gets += u64::from(report_type == 0);
                    metrics.requests += 1;
                    let (status, bytes) =
                        get_reply(report_id, report_type, &metrics.last_feature_command);
                    session.send(ProviderFrame::HidGetReportReply {
                        request_id,
                        status,
                        bytes,
                    })?;
                    metrics.replies += 1;
                }
                RawReverseEvent::HidSetReportRequest {
                    request_id,
                    report_id,
                    report_type,
                    bytes,
                } => {
                    metrics.feature_sets += u64::from(report_type == 0);
                    metrics.requests += 1;
                    let status = set_reply(report_id, report_type, &bytes);
                    if status == 0 {
                        metrics.lizard_disable_seen |=
                            is_kernel_lizard_disable(Some(report_id), &bytes);
                        metrics.last_feature_command = bytes[1..].to_vec();
                    }
                    session.send(ProviderFrame::HidSetReportReply { request_id, status })?;
                    metrics.replies += 1;
                }
                RawReverseEvent::HidLifecycle(gr_hid::Lifecycle::Open) => metrics.opens += 1,
                RawReverseEvent::HidLifecycle(gr_hid::Lifecycle::Close) => metrics.closes += 1,
                _ => {}
            }
        }
    }
}

fn strip_report_id(bytes: &[u8]) -> &[u8] {
    if bytes.first() == Some(&1) {
        &bytes[1..]
    } else {
        bytes
    }
}

fn is_kernel_lizard_disable(report_id: Option<u8>, bytes: &[u8]) -> bool {
    let payload = strip_report_id(bytes);
    report_id == Some(1)
        && payload.first() == Some(&0x87)
        && payload.get(1).is_some_and(|length| {
            let length = usize::from(*length);
            length % 3 == 0
                && payload.get(2..2 + length).is_some_and(|settings| {
                    settings.chunks_exact(3).any(|setting| setting == [9, 0, 0])
                })
        })
}

// Raw UHID types are FEATURE=0, OUTPUT=1, INPUT=2. SET buffers include
// the numbered ID; GET replies must likewise contain exactly one ID byte.
fn set_reply(id: u8, kind: u8, wire: &[u8]) -> i16 {
    if kind != 0 || id != 1 {
        return -95;
    }
    if wire.len() < 3 || wire.len() > 65 || wire[0] != id || usize::from(wire[2]) > wire.len() - 3 {
        return -22;
    }
    if !matches!(wire[1], 0x83 | 0x87 | 0xae) {
        return -95;
    }
    0
}

fn get_reply(id: u8, kind: u8, command: &[u8]) -> (i16, Vec<u8>) {
    if kind != 0 || id != 1 {
        return (-95, Vec::new());
    }
    let payload: &[u8] = match command {
        [0xae, ..] => &[0xae, 1, 1, 0], // Synthetic empty identity.
        [0x83, ..] => &[0x83, 2, 0, 0], // Synthetic empty attribute list.
        [0x87, ..] => &[0x87, 1, 0],
        _ => return (-95, Vec::new()),
    };
    let mut wire = vec![1];
    wire.extend_from_slice(payload);
    (0, wire)
}

fn fixture(name: &str) -> Result<Vec<u8>, io::Error> {
    let text = match name {
        "steam-c6-neutral-42.hex" => {
            include_str!("../protocol-research/fixtures/steam-c6-neutral-42.hex")
        }
        "steam-c6-buttons-all-42.hex" => {
            include_str!("../protocol-research/fixtures/steam-c6-buttons-all-42.hex")
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unknown probe fixture",
            ));
        }
    };
    text.split_ascii_whitespace()
        .map(|byte| u8::from_str_radix(byte, 16).map_err(io::Error::other))
        .collect()
}

fn send_fixture(
    session: &mut dyn NativeProviderSession,
    report: &[u8],
) -> Result<(), ProviderError> {
    let Some((&report_id, bytes)) = report.split_first() else {
        return Err(ProviderError::Open {
            reason: "empty input fixture".into(),
        });
    };
    session.send(ProviderFrame::HidInput {
        report_id: Some(report_id),
        bytes: bytes.to_vec(),
    })
}

fn find_hid_nodes(physical_path: &str) -> io::Result<Vec<HidNode>> {
    let mut nodes = Vec::new();
    for entry in fs::read_dir("/sys/bus/hid/devices")? {
        let path = entry?.path();
        let uevent = fs::read_to_string(path.join("uevent")).unwrap_or_default();
        if !uevent
            .lines()
            .any(|line| line == format!("HID_PHYS={physical_path}"))
        {
            continue;
        }
        let driver = fs::read_link(path.join("driver"))
            .ok()
            .and_then(|link| {
                link.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| "none".into());
        nodes.push(HidNode {
            hidraw: named_children(&path.join("hidraw"), "hidraw")?,
            events: event_children(&path.join("input"))?,
            driver,
        });
    }
    Ok(nodes)
}

fn named_children(path: &Path, prefix: &str) -> io::Result<Vec<PathBuf>> {
    match fs::read_dir(path) {
        Ok(entries) => Ok(entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|entry| {
                entry
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with(prefix))
            })
            .collect()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(error),
    }
}

fn event_children(input: &Path) -> io::Result<Vec<PathBuf>> {
    let mut events = Vec::new();
    for device in match fs::read_dir(input) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(events),
        Err(error) => return Err(error),
    } {
        let device = device?.path();
        events.extend(named_children(&device.join("event"), "event")?);
    }
    Ok(events)
}

fn print_inventory(case: Case, nodes: &[HidNode]) {
    let hidraw: Vec<_> = nodes
        .iter()
        .flat_map(|node| &node.hidraw)
        .filter_map(|path| path.file_name())
        .map(|name| format!("/dev/{}", name.to_string_lossy()))
        .collect();
    println!(
        "{{\"record_type\":\"kernel_inventory\",\"case\":\"{}\",\"hid_devices\":{},\"drivers\":[{}],\"hidraw\":[{}],\"evdev_nodes\":{}}}",
        case.name(),
        nodes.len(),
        nodes
            .iter()
            .map(|node| format!("\"{}\"", node.driver))
            .collect::<Vec<_>>()
            .join(","),
        hidraw
            .iter()
            .map(|path| format!("\"{path}\""))
            .collect::<Vec<_>>()
            .join(","),
        nodes.iter().map(|node| node.events.len()).sum::<usize>()
    );
}

fn spawn_sdl_probe(hidraw: &Path) -> io::Result<ProbeChild> {
    let Some(probe) = std::env::var_os("VIRTUALGAMEPAD_SDL_PROBE") else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "VIRTUALGAMEPAD_SDL_PROBE is not set",
        ));
    };
    let path = hidraw
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid hidraw path"))?;
    let device = Path::new("/dev").join(path);
    let child = Command::new(probe)
        .process_group(0)
        .arg(&device)
        .arg("1000")
        .env("SDL_JOYSTICK_HIDAPI", "1")
        .env("SDL_JOYSTICK_HIDAPI_STEAM", "1")
        .env("SDL_JOYSTICK_HIDAPI_PS5", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(ProbeChild::new(child))
}

fn poll_sdl_probe(child: &mut Option<ProbeChild>, case: Case, opened: &mut bool) -> io::Result<()> {
    let Some(process) = child.as_mut() else {
        return Ok(());
    };
    let Some(status) = process.child.try_wait()? else {
        if process.started.elapsed() >= Duration::from_secs(15) {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "SDL probe exceeded 15 seconds",
            ));
        }
        return Ok(());
    };
    let output = process.output()?;
    println!(
        "{{\"record_type\":\"sdl_probe\",\"case\":\"{}\",\"exit_code\":{},\"output\":{}}}",
        case.name(),
        status.code().unwrap_or(-1),
        json_string(&output)
    );
    *opened = true; // Attempt completed; opening is reported separately by the child.
    *child = None;
    Ok(())
}

fn stop_sdl_probe(child: &mut Option<ProbeChild>) -> io::Result<()> {
    if let Some(mut process) = child.take() {
        process.stop()?;
    }
    Ok(())
}

fn run_hidapi_probe(hidraw: &Path, case: Case) -> Result<(), Box<dyn Error>> {
    let Some(probe) = std::env::var_os("VIRTUALGAMEPAD_HIDAPI_PROBE") else {
        return Ok(());
    };
    let path = hidraw.file_name().ok_or("invalid hidraw path")?;
    let device = Path::new("/dev").join(path);
    let mut child = ProbeChild::new(
        Command::new(probe)
            .process_group(0)
            .arg(&device)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?,
    );
    let status = loop {
        if let Some(status) = child.child.try_wait()? {
            break status;
        }
        if child.started.elapsed() >= Duration::from_secs(5) {
            return Err("HIDAPI metadata timeout".into());
        }
        thread::sleep(Duration::from_millis(2));
    };
    let output = child.output()?;
    println!(
        "{{\"record_type\":\"hidapi_probe\",\"case\":\"{}\",\"exit_code\":{},\"metadata\":{}}}",
        case.name(),
        status.code().unwrap_or(-1),
        json_string(&output)
    );
    Ok(())
}

struct ProbeChild {
    child: Child,
    reader: Option<thread::JoinHandle<io::Result<String>>>,
    started: Instant,
}
impl ProbeChild {
    fn new(mut child: Child) -> Self {
        let stdout = child.stdout.take();
        let reader = Some(thread::spawn(move || {
            let mut output = String::new();
            if let Some(mut stdout) = stdout {
                stdout.read_to_string(&mut output)?;
            }
            Ok(output)
        }));
        Self {
            child,
            reader,
            started: Instant::now(),
        }
    }
    fn output(&mut self) -> io::Result<String> {
        self.reader
            .take()
            .ok_or_else(|| io::Error::other("output already consumed"))?
            .join()
            .map_err(|_| io::Error::other("output reader panicked"))?
    }
    fn stop(&mut self) -> io::Result<()> {
        if self.child.try_wait()?.is_none() {
            let _ = Command::new("kill")
                .args(["-KILL", "--", &format!("-{}", self.child.id())])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()?;
            self.child.wait()?;
        }
        Ok(())
    }
}
impl Drop for ProbeChild {
    fn drop(&mut self) {
        let _ = self.stop();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

fn exact_node_access(raw: &Path, case: Case) -> io::Result<bool> {
    let name = raw
        .file_name()
        .ok_or_else(|| io::Error::other("invalid hidraw name"))?;
    let device = Path::new("/dev").join(name);
    let result = OpenOptions::new().read(true).write(true).open(&device);
    let accessible = result.is_ok();
    println!(
        "{{\"record_type\":\"exact_node_access\",\"case\":\"{}\",\"read_write\":{},\"error_kind\":{}}}",
        case.name(),
        accessible,
        result.err().map_or_else(
            || "null".into(),
            |e| json_string(&format!("{:?}", e.kind()))
        )
    );
    Ok(accessible)
}

fn verify_removed(physical_path: &str) -> io::Result<()> {
    let deadline = Instant::now() + Duration::from_secs(3);
    while !find_hid_nodes(physical_path)?.is_empty() {
        if Instant::now() >= deadline {
            return Err(io::Error::other("owned HID nodes survived close"));
        }
        thread::sleep(Duration::from_millis(10));
    }
    println!(
        "{{\"record_type\":\"cleanup\",\"owned_nodes_removed\":true,\"close_repeated\":true}}"
    );
    Ok(())
}

fn json_string(value: &str) -> String {
    let mut result = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            value if value.is_control() => result.push(' '),
            value => result.push(value),
        }
    }
    result.push('"');
    result
}

fn run_dualsense_control(seconds: u64) -> Result<(), Box<dyn Error>> {
    let options = CreationOptions::new(RealizationId::LINUX_UHID_USB);
    let mut controller = create_dualsense(options)?;
    let physical_path = controller
        .association()
        .components()
        .first()
        .and_then(|component| component.requested_physical_path())
        .map(str::to_owned)
        .ok_or("DualSense has no requested UHID physical path")?;
    let deadline = Instant::now() + Duration::from_secs(seconds);
    let mut last_inventory_at = Instant::now();
    let mut last_inventory_nodes = None;
    let mut hidapi_probed = false;
    let mut sdl_probed = false;
    let mut sdl_child = None;
    while Instant::now() < deadline {
        controller.service(&mut |_| {})?;
        if last_inventory_at.elapsed() >= Duration::from_secs(5) {
            let nodes = find_hid_nodes(&physical_path)?;
            if !nodes.is_empty() && last_inventory_nodes.as_ref() != Some(&nodes) {
                print_inventory(Case::DualSense, &nodes);
                last_inventory_nodes = Some(nodes.clone());
            }
            if sdl_child.is_none()
                && !sdl_probed
                && let Some(raw) = nodes.iter().flat_map(|node| &node.hidraw).next()
            {
                let access = exact_node_access(raw, Case::DualSense)?;
                if access
                    && !hidapi_probed
                    && std::env::var_os("VIRTUALGAMEPAD_HIDAPI_PROBE").is_some()
                {
                    run_hidapi_probe(raw, Case::DualSense)?;
                    hidapi_probed = true;
                }
                if access && std::env::var_os("VIRTUALGAMEPAD_SDL_PROBE").is_some() {
                    sdl_child = Some(spawn_sdl_probe(raw)?);
                }
            }
            last_inventory_at = Instant::now();
        }
        poll_sdl_probe(&mut sdl_child, Case::DualSense, &mut sdl_probed)?;
        thread::sleep(Duration::from_millis(2));
    }
    stop_sdl_probe(&mut sdl_child)?;
    controller.close();
    controller.close();
    verify_removed(&physical_path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_lizard_disable_with_and_without_wire_report_id() {
        let command = [0x87, 0x06, 0x09, 0x00, 0x00, 0x47, 0, 0];
        let wire = [0x01, 0x87, 0x06, 0x09, 0x00, 0x00, 0x47, 0, 0];
        assert!(is_kernel_lizard_disable(Some(1), &command));
        assert!(is_kernel_lizard_disable(Some(1), &wire));
        assert!(!is_kernel_lizard_disable(Some(2), &wire));
        assert!(!is_kernel_lizard_disable(
            Some(1),
            &[0x87, 0x06, 0x08, 0, 0, 0x47]
        ));
        assert_eq!(strip_report_id(&wire), &command);
    }
    #[test]
    fn types_framing_and_exact_status() {
        for kind in [0, 1, 2, 99] {
            for command in [0x83, 0x87, 0xae] {
                assert_eq!(
                    set_reply(1, kind, &[1, command, 0]),
                    if kind == 0 { 0 } else { -95 }
                );
                let (status, wire) = get_reply(1, kind, &[command, 0]);
                assert_eq!(status, if kind == 0 { 0 } else { -95 });
                if status == 0 {
                    assert_eq!(&wire[..2], &[1, command]);
                } else {
                    assert!(wire.is_empty());
                }
            }
        }
        assert_eq!(set_reply(2, 0, &[2, 0x83, 0]), -95);
        for wire in [
            vec![],
            vec![1],
            vec![0x83, 0],
            vec![2, 0x83, 0],
            vec![1, 0x87, 4, 9],
        ] {
            assert_eq!(set_reply(1, 0, &wire), -22);
        }
        assert_eq!(set_reply(1, 0, &[1, 0xff, 0]), -95);
        assert_eq!(get_reply(1, 0, &[]), (-95, vec![]));
        assert!(is_kernel_lizard_disable(Some(1), &[1, 0x87, 3, 9, 0, 0]));
    }

    struct Fake {
        events: std::collections::VecDeque<RawReverseEvent>,
        replies: Vec<ProviderFrame>,
        pending: Option<u32>,
    }
    impl NativeProviderSession for Fake {
        fn send(&mut self, frame: ProviderFrame) -> Result<(), ProviderError> {
            let id = match &frame {
                ProviderFrame::HidGetReportReply { request_id, .. }
                | ProviderFrame::HidSetReportReply { request_id, .. } => *request_id,
                _ => panic!("unexpected frame"),
            };
            assert_eq!(
                self.pending.take(),
                Some(id),
                "reply must be exact and unique"
            );
            self.replies.push(frame);
            Ok(())
        }
        fn drain_reverse_events(
            &mut self,
            out: &mut dyn gr_realization_api::ProviderReverseEventSink,
        ) -> Result<(), ProviderError> {
            assert!(
                self.pending.is_none(),
                "host request left pending across polling cycle"
            );
            let Some(event) = self.events.pop_front() else {
                return Err(ProviderError::WouldBlock);
            };
            self.pending = match &event {
                RawReverseEvent::HidGetReportRequest { request_id, .. }
                | RawReverseEvent::HidSetReportRequest { request_id, .. } => Some(*request_id),
                _ => None,
            };
            out.push(gr_realization_api::ProviderReverseEvent {
                session: RealizationSessionId(1),
                sequence: 1,
                event,
            });
            Ok(())
        }
        fn readiness(&self) -> gr_realization_api::EventReadiness {
            gr_realization_api::EventReadiness::AlwaysPoll
        }
        fn diagnostics(&self) -> gr_realization_api::ProviderDiagnostics {
            unreachable!()
        }
        fn close(&mut self) -> Result<(), ProviderError> {
            assert!(self.pending.is_none());
            Ok(())
        }
    }
    #[test]
    fn sequence_acknowledges_every_request_before_next_poll() {
        let mut events = std::collections::VecDeque::new();
        for kind in [0, 1, 2, 99] {
            events.push_back(RawReverseEvent::HidSetReportRequest {
                request_id: u32::from(kind) * 2,
                report_id: 1,
                report_type: kind,
                bytes: vec![1, 0x83, 0],
            });
            events.push_back(RawReverseEvent::HidGetReportRequest {
                request_id: u32::from(kind) * 2 + 1,
                report_id: 1,
                report_type: kind,
            });
        }
        let mut session = Fake {
            events,
            replies: vec![],
            pending: None,
        };
        let mut metrics = Metrics::default();
        drain_events(&mut session, &mut metrics).unwrap();
        drain_events(&mut session, &mut metrics).unwrap();
        assert_eq!(
            (
                metrics.requests,
                metrics.replies,
                metrics.feature_sets,
                metrics.feature_gets
            ),
            (8, 8, 1, 1)
        );
        for (index, frame) in session.replies.iter().enumerate() {
            match frame {
                ProviderFrame::HidSetReportReply { status, .. }
                | ProviderFrame::HidGetReportReply { status, .. } => {
                    assert_eq!(*status, if index < 2 { 0 } else { -95 })
                }
                _ => panic!("unexpected reply"),
            }
        }
        session.close().unwrap();
        session.close().unwrap();
    }
    #[test]
    fn failed_consumer_scope_reaps_process_and_its_children() {
        let child = Command::new("sh")
            .args(["-c", "sleep 60 & wait"])
            .process_group(0)
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let pid = child.id();
        let result: io::Result<()> = (|| {
            let _guard = ProbeChild::new(child);
            Err(io::Error::other("simulated consumer failure"))
        })();
        assert!(result.is_err());
        assert!(!Path::new(&format!("/proc/{pid}")).exists());
    }
}
