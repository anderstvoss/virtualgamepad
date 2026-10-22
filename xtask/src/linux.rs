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
use virtualgamepad::{
    ControllerSurfaceInfo, CreationOptions, DigitalControlUpdate, InputTopology, RealizationId,
};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Capture(PathBuf);
impl Drop for Capture {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

macro_rules! controllers {
    ($(($variant:ident,$ty:ident,$create:ident,$id:literal)),+ $(,)?) => {
        pub enum Controller { $($variant(virtualgamepad::$ty),)+ Wii(gr_curated_controllers::wii_remote::WiiRemoteController) }
        impl Controller {
            fn create(id:&str,target:RealizationId)->Result<Self,String>{
                if id == "virtualgamepad.wii-remote" {
                    return gr_curated_controllers::wii_remote::create_wii_remote(target, NEXT.fetch_add(1,Ordering::Relaxed)).map(Self::Wii).map_err(|e|e.to_string());
                }
                match id { $($id=>virtualgamepad::$create(CreationOptions::new(target)).map(Self::$variant).map_err(|e|e.to_string()),)+ _=>Err("unmapped compiled family".into()) }
            }
            fn physical_paths(&self)->Vec<String> { match self { $(Self::$variant(c)=>c.association().components().iter().filter_map(|c|c.requested_physical_path().map(str::to_owned)).collect(),)+ Self::Wii(c)=>c.association().requested_physical_path.iter().cloned().chain(c.association().companions.iter().filter_map(|c|c.requested_physical_path.clone())).collect() } }
            fn topology(&self)->&InputTopology { match self { $(Self::$variant(c)=>c.surface().common_surface().input_topology(),)+ Self::Wii(c)=>c.surface().input_topology() } }
            fn digital(&mut self,update:DigitalControlUpdate)->Result<(),String>{match self {$(Self::$variant(c)=>c.set_digital(update).map_err(|e|e.to_string()),)+ Self::Wii(c)=>c.set_digital(update).map_err(|e|e.to_string())}}
            fn commit(&mut self)->Result<(),String>{match self {$(Self::$variant(c)=>c.commit().map_err(|e|e.to_string()),)+ Self::Wii(c)=>c.commit().map_err(|e|e.to_string())}}
            fn service(&mut self)->Result<(),String>{match self {$(Self::$variant(c)=>c.service(&mut |_|{}).map_err(|e|e.to_string()),)+ Self::Wii(c)=>c.service(&mut |_|{}).map_err(|e|e.to_string())}}
            fn close(&mut self){match self {$(Self::$variant(c)=>c.close(),)+ Self::Wii(c)=>c.close()}}
        }
        pub const MAPPED_IDS:&[&str]=&[$($id,)+ "virtualgamepad.wii-remote"];
    };
}
controllers! {
    (DualSense,DualSenseController,create_dualsense,"virtualgamepad.dualsense"),
    (DualShock4,DualShock4Controller,create_dualshock4,"virtualgamepad.dualshock4"),
    (SwitchPro,SwitchProController,create_switch_pro,"virtualgamepad.switch-pro"),
    (Xbox360,Xbox360Controller,create_xbox360,"virtualgamepad.xbox360"),
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
fn wait_child(
    child: &mut Child,
    controller: &mut Controller,
    peers: &mut [&mut Session],
) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        for peer in peers.iter_mut() {
            peer.service.service(|| peer.controller.service());
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                return if status.success() {
                    Ok(())
                } else {
                    Err(format!("SDL probe exited {status}"))
                };
            }
            Ok(None) => {}
            Err(error) => return Err(error.to_string()),
        }
        if let Err(error) = controller.service() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
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
                    if stage=="node_present" && !nodes.is_empty(){
                        s.nodes=nodes;
                        return Ok(json!({"poll_interval_ns":1_000_000,"devices":s.nodes.iter().map(|p|json!({"path":p,"bound_driver":bound_driver(p),"driver_reason":if bound_driver(p).is_none(){Some("no bound driver observed")}else{None}})).collect::<Vec<_>>() }));
                    }
                    if stage=="node_gone" && nodes.is_empty(){return Ok(json!({"poll_interval_ns":1_000_000}));}
                    if Instant::now()>=deadline{return Err(format!("{stage} not observed within two seconds"));}
                    if stage=="node_present"{s.controller.service()?;}
                    thread::sleep(Duration::from_millis(1));
                }
            }
            "consumer_open"=>{
                if s.nodes.is_empty(){return Err("no owned node observed".into());}
                for node in &s.nodes{s.consumers.push(OpenOptions::new().read(true).custom_flags(libc::O_NONBLOCK).open(node).map_err(|e|e.to_string())?);}
                Ok(json!({"consumer":"harness evdev/hidraw","nodes_opened":s.consumers.len()}))
            }
            "first_neutral"=>Err("no validated consumer neutral-report oracle for this family; API commit is not consumer delivery".into()),
            "sdl_ready"=>{
                if !self.sdl{return Err("SDL probe not requested".into());}
                let node=s.nodes.iter().find(|p|p.starts_with("/dev/input")).or_else(||s.nodes.first()).ok_or("no owned node for SDL")?;
                let path = std::env::temp_dir().join(format!("virtualgamepad-acceptance-{}-{}.json", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
                let file = OpenOptions::new().write(true).create_new(true).open(&path).map_err(|e|e.to_string())?;
                let capture = Capture(path);
                let mut child = Command::new("bash")
                    .arg(self.root.join("scripts/run-sdl3-gamepad-probe.sh"))
                    .arg(node).arg("100").stdout(Stdio::from(file)).stderr(Stdio::null())
                    .spawn().map_err(|e|e.to_string())?;
                wait_child(&mut child, &mut s.controller, peers)?;
                let text = fs::read_to_string(&capture.0).map_err(|e|e.to_string())?;
                let record = text.lines().filter_map(|line|serde_json::from_str::<Value>(line).ok()).find(|r|r["consumer"]=="SDL").ok_or("SDL probe emitted no observation")?;
                if record["opened"]["value"] != true { return Err("SDL did not observe this exact device opened".into()); }
                Ok(json!({"consumer":"SDL","timestamp_scope":"probe completion upper bound; discovery may occur earlier","observation":record}))
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
