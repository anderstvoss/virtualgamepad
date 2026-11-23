//! Shared CLI and attachment identity for the Real USB audio examples.
use std::{error::Error, fs, io, ops::RangeInclusive, path::Path, process::Command};
use virtualgamepad::{ComponentKind, ControllerAssociation, RealizationId};

pub struct Args {
    pub family: &'static str,
    options: Vec<String>,
}

impl Args {
    pub fn parse() -> Result<Self, Box<dyn Error>> {
        let mut args = std::env::args().skip(1);
        let family = match args.next().as_deref() {
            Some("dualsense") => "dualsense",
            Some("ds4" | "dualshock4") => "dualshock4",
            Some("xbox360") => "xbox360",
            _ => {
                return Err(
                    "expected FAMILY (dualsense, ds4 or xbox360), followed by options".into(),
                );
            }
        };
        Ok(Self {
            family,
            options: args.collect(),
        })
    }

    #[allow(dead_code, reason = "only the streaming examples use numeric options")]
    pub fn number(
        &mut self,
        flag: &str,
        default: u64,
        range: RangeInclusive<u64>,
    ) -> Result<u64, Box<dyn Error>> {
        let Some(index) = self.options.iter().position(|option| option == flag) else {
            return Ok(default);
        };
        self.options.remove(index);
        if index == self.options.len() {
            return Err(format!("{flag} requires a value").into());
        }
        let value = self.options.remove(index).parse()?;
        if !range.contains(&value) {
            return Err(format!("{flag} must be in {range:?}").into());
        }
        Ok(value)
    }

    #[allow(
        dead_code,
        reason = "only the root smoke example selects ownership on the CLI"
    )]
    pub fn flag(&mut self, flag: &str) -> bool {
        if let Some(index) = self.options.iter().position(|option| option == flag) {
            self.options.remove(index);
            true
        } else {
            false
        }
    }

    pub fn finish(&self) -> Result<(), Box<dyn Error>> {
        if let Some(option) = self.options.first() {
            return Err(format!("unexpected option: {option}").into());
        }
        Ok(())
    }
}

pub struct Attachment {
    port: u32,
    instance: String,
    generation: u64,
}

impl Attachment {
    pub fn discover(association: &ControllerAssociation) -> Result<Self, Box<dyn Error>> {
        if association.realization() != RealizationId::LINUX_USBIP_USB_AUDIO {
            return Err("expected a Real USB audio controller".into());
        }
        let parent = association
            .components()
            .iter()
            .find(|component| component.kind() == ComponentKind::Input)
            .and_then(|component| component.observed_host_path())
            .map(Path::new)
            .ok_or("controller has no observed USB parent")?;
        let bus = parent
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("invalid USB parent")?;
        if !parent
            .components()
            .any(|part| part.as_os_str() == "vhci_hcd.0")
            || Path::new("/sys/bus/usb/devices").join(bus).canonicalize()? != parent
        {
            return Err("observed USB parent is not the live VHCI device".into());
        }
        let status = fs::read_to_string("/sys/devices/platform/vhci_hcd.0/status")?;
        let serial = fs::read_to_string(parent.join("serial"))?;
        Ok(Self::parse(bus, &status, serial.trim_end_matches('\n'))?)
    }

    fn parse(bus: &str, status: &str, serial: &str) -> io::Result<Self> {
        let mut ports = status.lines().skip(1).filter_map(|line| {
            let fields: Vec<_> = line.split_whitespace().collect();
            match fields.as_slice() {
                ["hs", port, "006", "003", _, _, device] if *device == bus => Some(*port),
                _ => None,
            }
        });
        let port = ports
            .next()
            .and_then(|port| port.parse().ok())
            .filter(|_| ports.next().is_none())
            .ok_or_else(|| {
                io::Error::other("USB parent has no unique active high-speed VHCI port")
            })?;
        let (instance, generation) = serial
            .strip_prefix("vg-")
            .and_then(|value| value.rsplit_once('-'))
            .filter(|(instance, generation)| {
                !instance.is_empty()
                    && instance.len() <= 32
                    && instance
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
                    && generation.len() == 16
                    && generation
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            })
            .and_then(|(instance, generation)| {
                let generation = u64::from_str_radix(generation, 16).ok()?;
                (generation != 0).then_some((instance, generation))
            })
            .ok_or_else(|| io::Error::other("live attachment has no valid broker serial"))?;
        Ok(Self {
            port,
            instance: instance.to_owned(),
            generation,
        })
    }

    pub fn checker(&self, family: &str, trial: Option<(u64, u64)>) -> Command {
        let mut command = Command::new("python3");
        command
            .arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/scripts/validate-usb-audio-live.py"
            ))
            .args(["--reserve-owned-card", "--profile", family, "--port"])
            .arg(self.port.to_string())
            .arg("--instance")
            .arg(&self.instance)
            .arg("--generation")
            .arg(self.generation.to_string());
        if let Some((seconds, trials)) = trial {
            command
                .arg("--seconds")
                .arg(seconds.to_string())
                .arg("--trials")
                .arg(trials.to_string());
        } else {
            command.arg("--prepare-only");
        }
        command
    }

    pub fn prepare(&self, family: &str) -> Result<(), Box<dyn Error>> {
        if !self.checker(family, None).status()?.success() {
            return Err("owned ALSA card preparation failed".into());
        }
        Ok(())
    }
}

// Keep in sync with the checker's indexed S16 marker format.
#[allow(
    dead_code,
    reason = "used by the duplex and native signal-flow examples"
)]
pub fn marker_frame(position: u64, channels: usize) -> [i16; 4] {
    let index = if channels == 1 {
        position / 2
    } else {
        position
    };
    let high = i16::try_from(index / 32_767 + 1).expect("bounded trial marker high");
    let low = i16::try_from(index % 32_767 + 1).expect("bounded trial marker low");
    if channels == 1 {
        [if position % 2 == 0 { high } else { -low }, 0, 0, 0]
    } else {
        [high, low, -high, -low]
    }
}

#[cfg(test)]
mod tests {
    use super::{Attachment, Path};

    const STATUS: &str = "hub port sta spd dev      sockfd local_busid
hs  0000 006 003 00001234 000001 3-1
hs  0001 006 003 00005678 000002 3-2
hs  0002 004 000 00000000 000000 0-0";

    #[test]
    fn checker_modes_use_identity_and_port_of_the_selected_attachment() {
        let first = Attachment::parse("3-1", STATUS, "vg-lab-a-0000000000000007").unwrap();
        let second = Attachment::parse("3-2", STATUS, "vg-lab-a-0000000000000012").unwrap();
        let prepare = first.checker("dualsense", None);
        assert_eq!(prepare.get_program(), "python3");
        assert!(
            Path::new(prepare.get_args().next().unwrap())
                .ends_with("scripts/validate-usb-audio-live.py")
        );
        assert_eq!(
            prepare.get_args().skip(1).collect::<Vec<_>>(),
            [
                "--reserve-owned-card",
                "--profile",
                "dualsense",
                "--port",
                "0",
                "--instance",
                "lab-a",
                "--generation",
                "7",
                "--prepare-only",
            ]
        );
        let trial = second.checker("dualshock4", Some((3, 2)));
        assert_eq!(
            trial.get_args().skip(1).collect::<Vec<_>>(),
            [
                "--reserve-owned-card",
                "--profile",
                "dualshock4",
                "--port",
                "1",
                "--instance",
                "lab-a",
                "--generation",
                "18",
                "--seconds",
                "3",
                "--trials",
                "2",
            ]
        );
    }

    #[test]
    fn closed_ambiguous_and_malformed_attachments_are_rejected() {
        let serial = "vg-lab-0000000000000007";
        assert!(Attachment::parse("0-0", STATUS, serial).is_err());
        let ambiguous = format!("{STATUS}\nhs 0003 006 003 00005678 000003 3-2");
        assert!(Attachment::parse("3-2", &ambiguous, serial).is_err());
        for invalid in [
            "vg-../foreign-0000000000000007",
            "vg-lab-0000000000000000",
            "vg-lab-000000000000000A",
            "vg-lab-7",
        ] {
            assert!(Attachment::parse("3-2", STATUS, invalid).is_err());
        }
    }
}
