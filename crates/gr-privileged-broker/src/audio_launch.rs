//! Privileged worker launch boundary. Only this fixed installed executable is used.
#![allow(unsafe_code)]
use std::{
    ffi::CString,
    fs::{self, File, OpenOptions},
    io,
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::{
            ffi::OsStrExt,
            fs::{MetadataExt, OpenOptionsExt},
            net::UnixStream,
            process::CommandExt,
        },
    },
    path::Path,
    process::{Child, Command, Stdio},
};
const WORKER: &str = "/usr/libexec/virtualgamepad/gr-audio-worker";

#[derive(Clone, Copy, Debug)]
pub enum Profile {
    DualSense,
    DualShock4,
    Xbox360,
}
impl Profile {
    pub fn from_tag(tag: u8) -> io::Result<Self> {
        match tag {
            1 => Ok(Self::DualSense),
            2 => Ok(Self::DualShock4),
            3 => Ok(Self::Xbox360),
            _ => Err(io::Error::other("unsupported compiled audio profile")),
        }
    }
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::DualSense => "dualsense",
            Self::DualShock4 => "dualshock4",
            Self::Xbox360 => "xbox360",
        }
    }
}
fn executable(path: &Path) -> io::Result<File> {
    for parent in path.ancestors().skip(1) {
        let m = fs::symlink_metadata(parent)?;
        if !m.is_dir() || m.uid() != 0 || m.mode() & 0o022 != 0 {
            return Err(io::Error::other("untrusted worker executable parent"));
        }
    }
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)?;
    let m = file.metadata()?;
    if !m.is_file() || m.uid() != 0 || m.mode() & 0o022 != 0 || m.mode() & 0o111 == 0 {
        return Err(io::Error::other(
            "worker must be an administrator-owned executable",
        ));
    }
    Ok(file)
}
fn duplicate(socket: &impl AsRawFd) -> io::Result<OwnedFd> {
    // SAFETY: the borrowed socket is live; fcntl returns a new owned descriptor.
    let fd = unsafe { libc::fcntl(socket.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 10) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: successful fcntl transfers this newly allocated descriptor to us.
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}
#[derive(Clone, Copy)]
pub struct Launch {
    pub profile: Profile,
    pub device: u32,
    pub generation: u64,
    pub identity: [u8; 6],
    pub uid: u32,
    pub gid: u32,
}
pub fn spawn(config: Launch, channels: &[UnixStream; 4]) -> io::Result<Child> {
    if config.uid == 0
        || config.gid == 0
        || config.uid == u32::MAX
        || config.gid == u32::MAX
        || config.device == 0
        || config.generation == 0
    {
        return Err(io::Error::other("invalid worker launch identity"));
    }
    required_capabilities(&fs::read_to_string("/proc/thread-self/status")?)?;
    let file = executable(Path::new(WORKER))?;
    let image = duplicate(&file)?;
    drop(file);
    // Execute the verified open inode. Replacing the path after validation
    // cannot substitute a different executable between verification and exec.
    let mut command = Command::new(format!("/proc/self/fd/{}", image.as_raw_fd()));
    let identity = config
        .identity
        .iter()
        .flat_map(|v| {
            const HEX: &[u8; 16] = b"0123456789abcdef";
            [
                char::from(HEX[usize::from(v >> 4)]),
                char::from(HEX[usize::from(v & 15)]),
            ]
        })
        .collect::<String>();
    command.args([
        config.profile.name().to_string(),
        config.device.to_string(),
        config.generation.to_string(),
        identity,
    ]);
    command
        .env_clear()
        .env("LANG", "C")
        .current_dir("/")
        .stdin(Stdio::from(duplicate(&channels[0])?))
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let descriptors = channels[1..]
        .iter()
        .map(duplicate)
        .collect::<io::Result<Vec<_>>>()?;
    command.args(descriptors.iter().map(|fd| fd.as_raw_fd().to_string()));
    let argv = std::iter::once(std::ffi::OsStr::new(WORKER))
        .chain(command.get_args())
        .map(|arg| CString::new(arg.as_bytes()).map_err(io::Error::other))
        .collect::<io::Result<Vec<_>>>()?;
    let env = [CString::new("LANG=C").map_err(io::Error::other)?];
    let image_fd = image.as_raw_fd();
    // SAFETY: getpid has no pointer arguments or preconditions.
    let parent = unsafe { libc::getpid() };
    // SAFETY: after fork this closure uses only fixed storage and async-signal-safe
    // syscalls. All formatting/allocation and descriptor duplication happened above.
    unsafe {
        command.pre_exec(move || {
            for source in &descriptors {
                // Keep the broker-chosen high descriptor number. Remapping into
                // low slots could overwrite Command's private exec-error pipe.
                if libc::fcntl(source.as_raw_fd(), libc::F_SETFD, 0) < 0 {
                    return Err(io::Error::last_os_error());
                }
            }
            for (resource, limit) in [
                (libc::RLIMIT_CORE, 0),
                (libc::RLIMIT_NOFILE, 64),
                (libc::RLIMIT_AS, 512 * 1024 * 1024),
            ] {
                let value = libc::rlimit {
                    rlim_cur: limit,
                    rlim_max: limit,
                };
                if libc::setrlimit(resource, &raw const value) != 0 {
                    return Err(io::Error::last_os_error());
                }
            }
            if libc::setgroups(0, std::ptr::null()) != 0
                || libc::setresgid(config.gid, config.gid, config.gid) != 0
                || libc::setresuid(config.uid, config.uid, config.uid) != 0
            {
                return Err(io::Error::last_os_error());
            }
            clear_capabilities()?;
            // Set after dropping UID: Linux clears PDEATHSIG on credential change.
            if libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0
                || libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL, 0, 0, 0) != 0
            {
                return Err(io::Error::last_os_error());
            }
            if libc::getppid() != parent || libc::geteuid() == 0 {
                return Err(io::Error::from_raw_os_error(libc::EPERM));
            }
            exec_image(image_fd, &argv, &env)
        });
    }
    let child = command.spawn();
    drop(image);
    child
}
fn required_capabilities(status: &str) -> io::Result<()> {
    let effective = status
        .lines()
        .find_map(|line| line.strip_prefix("CapEff:\t"))
        .and_then(|value| u64::from_str_radix(value, 16).ok())
        .ok_or_else(|| io::Error::other("cannot inspect broker effective capabilities"))?;
    for (bit, name) in [(6, "CAP_SETGID"), (7, "CAP_SETUID"), (21, "CAP_SYS_ADMIN")] {
        if effective & (1 << bit) == 0 {
            return Err(io::Error::other(format!(
                "broker is missing effective {name}; update the installed systemd capability configuration"
            )));
        }
    }
    Ok(())
}
fn clear_capabilities() -> io::Result<()> {
    #[repr(C)]
    struct Header {
        version: u32,
        pid: i32,
    }
    let header = Header {
        version: 0x2008_0522,
        pid: 0,
    };
    // Two Linux capability records: effective, permitted, inheritable (u32 each).
    let empty = [0_u32; 6];
    // SAFETY: fixed Linux v3 capability ABI, live buffers of exact required size.
    if unsafe { libc::syscall(libc::SYS_capset, &raw const header, empty.as_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
// Execute the verified descriptor directly. Resolving /proc/self/fd after a UID
// transition can fail because Linux resets dumpability and procfs ownership.
fn exec_image(fd: libc::c_int, args: &[CString], env: &[CString]) -> io::Result<()> {
    if args.len() >= 16 || env.len() >= 16 {
        return Err(io::Error::from_raw_os_error(libc::E2BIG));
    }
    let mut arguments = [std::ptr::null(); 16];
    let mut envp = [std::ptr::null(); 16];
    for (pointer, value) in arguments.iter_mut().zip(args) {
        *pointer = value.as_ptr();
    }
    for (pointer, value) in envp.iter_mut().zip(env) {
        *pointer = value.as_ptr();
    }
    // SAFETY: descriptor is live, pointer arrays are terminated and C strings
    // remain live. fexecve is async-signal-safe and success never returns.
    unsafe {
        libc::fexecve(fd, arguments.as_ptr(), envp.as_ptr());
    }
    Err(io::Error::last_os_error())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_effective_launch_capability_has_actionable_error() {
        required_capabilities("CapEff:\t00000000002000c0\n").unwrap();
        let error = required_capabilities("CapEff:\t0000000000200040\n").unwrap_err();
        assert!(error.to_string().contains("CAP_SETUID"));
        assert!(required_capabilities("CapBnd:\t00000000002000c0\n").is_err());
    }
    #[test]
    fn worker_exec_has_no_effective_permitted_or_inheritable_capabilities() {
        let image = File::open("/usr/bin/cat").unwrap();
        let args = [
            CString::new("cat").unwrap(),
            CString::new("/proc/self/status").unwrap(),
        ];
        let mut command = Command::new("/nonexistent-not-used");
        // SAFETY: only capability dropping and descriptor execution occur after fork.
        unsafe {
            command.pre_exec(move || {
                clear_capabilities()?;
                exec_image(image.as_raw_fd(), &args, &[])
            });
        }
        let output = command.output().unwrap();
        assert!(output.status.success());
        let status = String::from_utf8(output.stdout).unwrap();
        for key in ["CapInh:", "CapPrm:", "CapEff:", "CapAmb:"] {
            let line = status.lines().find(|line| line.starts_with(key)).unwrap();
            assert_eq!(line.split_whitespace().nth(1), Some("0000000000000000"));
        }
    }
    #[test]
    fn executes_verified_descriptor_without_resolving_command_path() {
        let image = File::open("/usr/bin/true").unwrap();
        let args = [CString::new("verified-worker").unwrap()];
        let mut command = Command::new("/nonexistent-not-used");
        // SAFETY: closure only calls the bounded async-signal-safe helper;
        // captured File keeps the descriptor alive through exec.
        unsafe {
            command.pre_exec(move || exec_image(image.as_raw_fd(), &args, &[]));
        }
        assert!(command.status().unwrap().success());
    }
    #[test]
    fn profiles_are_compiled_and_unprivileged_ids_are_required_before_resources() {
        assert!(Profile::from_tag(0).is_err());
        assert!(Profile::from_tag(4).is_err());
        let channels = std::array::from_fn(|_| UnixStream::pair().unwrap().0);
        for (uid, gid) in [(0, 1), (1, 0), (u32::MAX, 1), (1, u32::MAX)] {
            assert!(
                spawn(
                    Launch {
                        profile: Profile::DualSense,
                        device: 1,
                        generation: 1,
                        identity: [0; 6],
                        uid,
                        gid
                    },
                    &channels
                )
                .is_err()
            );
        }
    }
    #[test]
    fn writable_or_symlinked_parents_cannot_supply_worker_code() {
        assert!(executable(Path::new("/tmp/not-a-worker")).is_err());
        assert!(executable(Path::new("/proc/self/exe")).is_err());
    }
}
