//! Bounded, ordinary-root controller producer for isolated consumer acceptance.
//! Run without `--apply` first. This producer does not establish Steam acceptance.
use std::{error::Error, path::PathBuf};
#[cfg(target_os = "linux")]
use std::{
    thread,
    time::{Duration, Instant},
};
#[cfg(target_os = "linux")]
use virtualgamepad::{
    CreationOptions, DigitalControlUpdate, DualSenseController, DualShock4Controller, FaceButton,
    RealizationId, SwitchProController, Xbox360Controller, create_dualsense, create_dualshock4,
    create_switch_pro, create_xbox360,
};

#[derive(Debug, PartialEq, Eq)]
struct Options {
    apply: bool,
    evdev: bool,
    seconds: u64,
    start_after: u64,
    remove_after: Option<u64>,
    control_directory: Option<PathBuf>,
}

impl Options {
    fn parse(arguments: impl IntoIterator<Item = String>) -> Result<Self, &'static str> {
        let mut result = Self {
            apply: false,
            evdev: false,
            seconds: 300,
            start_after: 120,
            remove_after: None,
            control_directory: None,
        };
        let mut arguments = arguments.into_iter();
        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "--apply" if !result.apply => result.apply = true,
                "--evdev" if !result.evdev => result.evdev = true,
                "--seconds" => {
                    result.seconds = arguments
                        .next()
                        .ok_or("missing duration")?
                        .parse()
                        .map_err(|_| "invalid duration")?;
                }
                "--start-after" => {
                    result.start_after = arguments
                        .next()
                        .ok_or("missing start delay")?
                        .parse()
                        .map_err(|_| "invalid start delay")?;
                }
                "--remove-after" => {
                    result.remove_after = Some(
                        arguments
                            .next()
                            .ok_or("missing removal delay")?
                            .parse()
                            .map_err(|_| "invalid removal delay")?,
                    );
                }
                "--control-directory" => {
                    result.control_directory =
                        Some(arguments.next().ok_or("missing control directory")?.into());
                }
                _ => return Err("unknown producer option"),
            }
        }
        if !(60..=900).contains(&result.seconds) || result.start_after > result.seconds - 50 {
            return Err("duration must be 60..900 seconds with at least 50 seconds after startup");
        }
        if result
            .remove_after
            .is_some_and(|delay| delay < result.start_after + 40 || delay > result.seconds - 10)
        {
            return Err(
                "removal must follow a complete four-family cycle and leave ten seconds for siblings",
            );
        }
        if result.apply && result.control_directory.is_none() {
            return Err("--apply requires a new private --control-directory");
        }
        Ok(result)
    }
}

/// Ten-second slots distinguish each family. Two seconds down, eight up.
/// `DualSense` removal occurs only during a neutral interval; siblings continue.
#[cfg(any(target_os = "linux", test))]
fn desired_inputs(seconds: u64, start_after: u64, remove_at: u64) -> [bool; 4] {
    let mut pressed = [false; 4];
    if seconds >= start_after {
        let elapsed = seconds - start_after;
        let family = usize::try_from((elapsed / 10) % 4).expect("four-family index fits usize");
        if elapsed % 10 < 2 && (family != 0 || seconds < remove_at) {
            pressed[family] = true;
        }
    }
    pressed
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = Options::parse(std::env::args().skip(1))?;
    println!(
        "proposed: four families; realization={}; duration={}; delay-after-arm={}; South pulses only; no touch/audio; inputs remain neutral until armed",
        if options.evdev { "evdev" } else { "hid" },
        options.seconds,
        options.start_after
    );
    if !options.apply {
        return Ok(());
    }
    #[cfg(not(target_os = "linux"))]
    return Err("this live producer requires Linux; no resources created".into());
    #[cfg(target_os = "linux")]
    run(&options)
}

#[cfg(target_os = "linux")]
#[allow(clippy::too_many_lines)] // Keep the four typed handles in one bounded ownership/service scope.
fn run(options: &Options) -> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::MetadataExt;
    if std::fs::metadata("/proc/self")?.uid() == 0 {
        return Err("run the consumer producer without root privileges".into());
    }
    let mut inbox = control::Inbox::create(
        options
            .control_directory
            .as_ref()
            .expect("validated directory"),
    )?;
    let target = if options.evdev {
        RealizationId::LINUX_UINPUT
    } else {
        RealizationId::LINUX_UHID_USB
    };
    let creation = CreationOptions::new(target);
    // Handle drops roll back earlier creations if any later family fails.
    let mut sony = create_dualsense(creation)?;
    let mut ds4 = create_dualshock4(creation)?;
    let mut switch = create_switch_pro(creation)?;
    let mut xbox = create_xbox360(creation)?;
    println!(
        "ready: producer-pid={}; all four controllers created",
        std::process::id()
    );
    let started = Instant::now();
    let mut pressed = [false; 4];
    let mut active = [true; 4];
    let mut observations = [0_u64; 4];
    let mut armed_start = None;
    let remove_at = options.remove_after.unwrap_or(options.seconds - 10);
    while started.elapsed() < Duration::from_secs(options.seconds) {
        let seconds = started.elapsed().as_secs();
        match inbox.poll()? {
            Some(control::Command::Stop) => break,
            Some(control::Command::Arm) if armed_start.is_none() => {
                let begin = seconds + options.start_after;
                if begin + 40 > remove_at {
                    return Err("not enough time for a complete input cycle before removal".into());
                }
                armed_start = Some(begin);
                println!("armed: first input at second={begin}");
            }
            _ => {}
        }
        let desired = armed_start.map_or([false; 4], |begin| {
            desired_inputs(seconds, begin, remove_at)
        });
        macro_rules! advance {
            ($controller:ident, $index:expr) => {
                if active[$index] {
                    if pressed[$index] != desired[$index] {
                        $controller.set_digital(DigitalControlUpdate::FaceButton {
                            button: FaceButton::South,
                            pressed: desired[$index],
                        })?;
                        $controller.commit()?;
                        pressed[$index] = desired[$index];
                        println!(
                            "input: family={} second={seconds} South={}",
                            stringify!($controller),
                            pressed[$index]
                        );
                    }
                    $controller.service(&mut |event| {
                        if observations[$index] < 64 {
                            println!("output: family={} event={event:?}", stringify!($controller));
                        }
                        observations[$index] = observations[$index].saturating_add(1);
                    })?;
                }
            };
        }
        advance!(sony, 0);
        advance!(ds4, 1);
        advance!(switch, 2);
        advance!(xbox, 3);
        if seconds >= remove_at && active[0] {
            sony.neutralize()?;
            sony.commit()?;
            sony.close();
            sony.close();
            active[0] = false;
            println!("removed: sony; three siblings remain serviced");
        }
        let delay = [
            if active[0] {
                sony.next_service_in()
            } else {
                None
            },
            ds4.next_service_in(),
            switch.next_service_in(),
            xbox.next_service_in(),
        ]
        .into_iter()
        .flatten()
        .fold(Duration::from_millis(4), Duration::min);
        thread::sleep(delay);
    }
    close_all(&mut sony, &mut ds4, &mut switch, &mut xbox, active[0])?;
    inbox.close()?;
    let truncated = observations.iter().any(|count| *count > 64);
    println!(
        "closed: all controllers; input-armed={}; output-counts={observations:?}; output-log-truncated={truncated}; consumer observations still required",
        armed_start.is_some()
    );
    if truncated {
        return Err("output log limit exceeded; repeat the affected consumer check".into());
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn close_all(
    sony: &mut DualSenseController,
    ds4: &mut DualShock4Controller,
    switch: &mut SwitchProController,
    xbox: &mut Xbox360Controller,
    sony_active: bool,
) -> Result<(), Box<dyn Error>> {
    if sony_active {
        sony.neutralize()?;
        sony.commit()?;
    }
    sony.close();
    ds4.neutralize()?;
    ds4.commit()?;
    ds4.close();
    switch.neutralize()?;
    switch.commit()?;
    switch.close();
    xbox.neutralize()?;
    xbox.commit()?;
    xbox.close();
    for diagnostics in [
        sony.diagnostics(),
        ds4.diagnostics(),
        switch.diagnostics(),
        xbox.diagnostics(),
    ] {
        if let Some(error) = diagnostics.last_error() {
            return Err(error.to_string().into());
        }
    }
    if [
        sony.dropped_output_events(),
        ds4.dropped_output_events(),
        switch.dropped_output_events(),
        xbox.dropped_output_events(),
    ]
    .into_iter()
    .any(|count| count != 0)
    {
        return Err(
            "controller output observations dropped; consumer evidence is incomplete".into(),
        );
    }
    Ok(())
}

#[cfg(target_os = "linux")]
mod control {
    use std::{
        fs, io,
        os::unix::{
            fs::{DirBuilderExt, FileTypeExt, MetadataExt},
            net::UnixDatagram,
        },
        path::{Path, PathBuf},
    };

    #[derive(Debug, PartialEq, Eq)]
    pub(super) enum Command {
        Arm,
        Stop,
    }

    fn parse(bytes: &[u8]) -> io::Result<Command> {
        match bytes {
            b"arm" | b"arm\n" => Ok(Command::Arm),
            b"stop" | b"stop\n" => Ok(Command::Stop),
            _ => Err(io::Error::other("unknown or oversized producer command")),
        }
    }

    fn identity(metadata: &fs::Metadata) -> (u64, u64, u32) {
        (metadata.dev(), metadata.ino(), metadata.uid())
    }

    pub(super) struct Inbox {
        directory: PathBuf,
        directory_identity: (u64, u64, u32),
        socket: Option<UnixDatagram>,
        socket_identity: Option<(u64, u64, u32)>,
        closed: bool,
    }

    impl Inbox {
        pub(super) fn create(directory: &Path) -> io::Result<Self> {
            fs::DirBuilder::new().mode(0o700).create(directory)?;
            let mut result = Self {
                directory: directory.into(),
                directory_identity: identity(&fs::symlink_metadata(directory)?),
                socket: None,
                socket_identity: None,
                closed: false,
            };
            let path = directory.join("control.sock");
            result.socket = Some(UnixDatagram::bind(&path)?);
            result.socket_identity = Some(identity(&fs::symlink_metadata(&path)?));
            result
                .socket
                .as_ref()
                .expect("bound socket")
                .set_nonblocking(true)?;
            println!(
                "control: {}; send arm only after verifying the isolated controller-test screen; stop cancels",
                path.display()
            );
            println!(
                "control-identities: directory={:?}; socket={:?}",
                result.directory_identity, result.socket_identity
            );
            Ok(result)
        }

        fn verify_directory(&self, private: bool) -> io::Result<()> {
            let metadata = fs::symlink_metadata(&self.directory)?;
            if !metadata.is_dir()
                || identity(&metadata) != self.directory_identity
                || (private && metadata.mode() & 0o077 != 0)
            {
                return Err(io::Error::other(
                    "producer control directory identity or permissions changed",
                ));
            }
            Ok(())
        }

        pub(super) fn poll(&self) -> io::Result<Option<Command>> {
            self.verify_directory(true)?;
            let metadata = fs::symlink_metadata(self.directory.join("control.sock"))?;
            if !metadata.file_type().is_socket()
                || Some(identity(&metadata)) != self.socket_identity
            {
                return Err(io::Error::other("producer socket identity changed"));
            }
            let mut bytes = [0_u8; 65];
            let socket = self
                .socket
                .as_ref()
                .ok_or_else(|| io::Error::other("producer control closed"))?;
            match socket.recv(&mut bytes) {
                Ok(count) => parse(&bytes[..count]).map(Some),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(None),
                Err(error) => Err(error),
            }
        }

        pub(super) fn close(&mut self) -> io::Result<()> {
            if self.closed {
                return Ok(());
            }
            self.socket.take();
            self.verify_directory(false)?;
            if let Some(expected) = self.socket_identity {
                let path = self.directory.join("control.sock");
                let current = fs::symlink_metadata(&path)?;
                if !current.file_type().is_socket() || identity(&current) != expected {
                    return Err(io::Error::other(
                        "producer socket identity changed; retained",
                    ));
                }
                fs::remove_file(path)?;
                self.socket_identity = None;
            }
            // Refuse a nonempty directory; never remove unexpected resources.
            fs::remove_dir(&self.directory)?;
            self.closed = true;
            Ok(())
        }
    }

    impl Drop for Inbox {
        fn drop(&mut self) {
            if let Err(error) = self.close() {
                eprintln!("producer control cleanup: {error}");
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        fn directory() -> PathBuf {
            let tick = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            std::env::temp_dir().join(format!("vg-probe-{}-{tick}", std::process::id()))
        }

        #[test]
        fn commands_are_bounded_and_do_not_arm_without_a_message() {
            let path = directory();
            let mut inbox = Inbox::create(&path).unwrap();
            assert_eq!(inbox.poll().unwrap(), None);
            let sender = UnixDatagram::unbound().unwrap();
            for (bytes, command) in [
                (b"arm".as_slice(), Command::Arm),
                (b"stop\n".as_slice(), Command::Stop),
            ] {
                sender.send_to(bytes, path.join("control.sock")).unwrap();
                assert_eq!(inbox.poll().unwrap(), Some(command));
            }
            for bytes in [
                vec![],
                vec![b'a'; 65],
                b"arm other".to_vec(),
                b"\xff".to_vec(),
            ] {
                sender.send_to(&bytes, path.join("control.sock")).unwrap();
                assert!(inbox.poll().is_err());
            }
            inbox.close().unwrap();
            inbox.close().unwrap();
            assert!(!path.exists());
        }

        #[test]
        fn changed_permissions_and_replaced_socket_fail_closed_without_foreign_cleanup() {
            use std::os::unix::fs::PermissionsExt;
            let path = directory();
            let mut inbox = Inbox::create(&path).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
            assert!(inbox.poll().is_err());
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
            let owned = path.join("owned.sock");
            fs::rename(path.join("control.sock"), &owned).unwrap();
            fs::write(path.join("control.sock"), b"sanitized foreign resource").unwrap();
            assert!(inbox.poll().is_err());
            assert!(inbox.close().is_err());
            assert_eq!(
                fs::read(path.join("control.sock")).unwrap(),
                b"sanitized foreign resource"
            );
            fs::remove_file(path.join("control.sock")).unwrap();
            fs::rename(owned, path.join("control.sock")).unwrap();
            inbox.close().unwrap();
            inbox.close().unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_pulses_release_and_removal_are_exact() {
        assert_eq!(desired_inputs(119, 120, 290), [false; 4]);
        for (second, family) in [(120, 0), (130, 1), (140, 2), (150, 3)] {
            let mut expected = [false; 4];
            expected[family] = true;
            assert_eq!(desired_inputs(second, 120, 290), expected);
            assert_eq!(desired_inputs(second + 1, 120, 290), expected);
            assert_eq!(desired_inputs(second + 2, 120, 290), [false; 4]);
        }
        assert_eq!(desired_inputs(320, 120, 290), [false; 4]);
        assert_eq!(desired_inputs(330, 120, 290), [false, true, false, false]);
    }

    #[test]
    fn options_default_to_preview_and_reject_unbounded_or_unknown_requests() {
        assert!(!Options::parse([]).unwrap().apply);
        let parsed = Options::parse(
            [
                "--apply",
                "--evdev",
                "--seconds",
                "60",
                "--start-after",
                "10",
                "--control-directory",
                "/synthetic/private-control",
            ]
            .map(str::to_owned),
        )
        .unwrap();
        assert!(parsed.apply && parsed.evdev);
        for arguments in [
            vec!["--seconds", "901"],
            vec!["--seconds", "0"],
            vec!["--seconds"],
            vec!["--start-after", "299"],
            vec!["--unknown"],
            vec!["--remove-after", "159"],
            vec!["--remove-after", "291"],
            vec!["--remove-after"],
            vec!["--apply"],
            vec!["--control-directory"],
        ] {
            assert!(Options::parse(arguments.into_iter().map(str::to_owned)).is_err());
        }
        let planned = Options::parse(
            [
                "--seconds",
                "900",
                "--start-after",
                "180",
                "--remove-after",
                "600",
            ]
            .map(str::to_owned),
        )
        .unwrap();
        assert_eq!(planned.remove_after, Some(600));
    }
}
