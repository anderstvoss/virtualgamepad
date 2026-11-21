//! Opt-in Alpha 1 host-run session holder using only the stable public API.
//! Output is an ephemeral controller association handshake, not an experiment record.
use std::{
    error::Error,
    io::{self, BufRead, Write},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
use virtualgamepad::{
    CreationOptions, DualSenseController, DualSenseOptions, DualShock4Controller,
    DualShock4Options, RealizationId, SwitchProController, SwitchProOptions,
    SwitchProOutputEvent, Xbox360Controller, Xbox360Options, create_dualsense,
    create_dualshock4, create_switch_pro, create_xbox360,
};

enum Session {
    DualSense(DualSenseController),
    DualShock4(DualShock4Controller),
    SwitchPro(SwitchProController),
    Xbox360(Xbox360Controller),
}

impl Session {
    fn open(family: &str, realization: RealizationId) -> Result<Self, Box<dyn Error>> {
        let options = CreationOptions::new(realization);
        Ok(match family {
            "dualsense" => Self::DualSense(create_dualsense(options, DualSenseOptions::default())?),
            "dualshock4" => Self::DualShock4(create_dualshock4(options, DualShock4Options::default())?),
            "switch-pro" => Self::SwitchPro(create_switch_pro(options, SwitchProOptions::default())?),
            "xbox360" => Self::Xbox360(create_xbox360(options, Xbox360Options::default())?),
            _ => return Err("unknown Alpha 1 family".into()),
        })
    }

    fn association_labels(&self) -> Vec<String> {
        let association = match self {
            Self::DualSense(c) => c.association(),
            Self::DualShock4(c) => c.association(),
            Self::SwitchPro(c) => c.association(),
            Self::Xbox360(c) => c.association(),
        };
        association.components().iter().filter_map(|part| {
            part.requested_physical_path().map(str::to_owned)
        }).collect()
    }

    fn service(&mut self) -> Result<(), Box<dyn Error>> {
        match self {
            Self::DualSense(c) => c.service(&mut |_| {})?,
            Self::DualShock4(c) => c.service(&mut |_| {})?,
            Self::Xbox360(c) => c.service(&mut |_| {})?,
            Self::SwitchPro(c) => c.service(&mut |event| {
                if let SwitchProOutputEvent::Output { report_id, bytes } = event {
                    match report_id {
                        Some(0x80) if !bytes.is_empty() => {
                            // Command number only: do not publish raw report contents.
                            println!("SW\t80\t{:02x}", bytes[0]);
                        }
                        Some(0x01) if bytes.len() >= 10 => {
                            // This is an outgoing subcommand, not a 0x21 ACK.
                            println!("SW\t01\t{:02x}", bytes[9]);
                        }
                        _ => {}
                    }
                }
            })?,
        }
        Ok(())
    }

    fn close(&mut self) {
        match self {
            Self::DualSense(c) => c.close(),
            Self::DualShock4(c) => c.close(),
            Self::SwitchPro(c) => c.close(),
            Self::Xbox360(c) => c.close(),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: alpha1_host_hold FAMILY REALIZATION".into());
    }
    let realization = match args[1].as_str() {
        "linux.uinput" => RealizationId::LINUX_UINPUT,
        "linux.uhid.usb" => RealizationId::LINUX_UHID_USB,
        "linux.uhid.bluetooth" => RealizationId::LINUX_UHID_BLUETOOTH,
        _ => return Err("undeclared realization".into()),
    };
    let mut session = Session::open(&args[0], realization)?;
    let labels = session.association_labels();
    if labels.is_empty() || labels.iter().any(|label| label.contains(['\n', '\t', '|'])) {
        session.close();
        return Err("controller association is unavailable".into());
    }
    // Private process pipe only. The Python harness uses this for ownership
    // attribution and never writes the labels into its JSON output.
    println!("READY\t{}", labels.join("|"));
    io::stdout().flush()?;

    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut command = String::new();
        let _ = io::stdin().lock().read_line(&mut command);
        let _ = sender.send(command);
    });
    let deadline = Instant::now() + Duration::from_secs(90);
    let result = loop {
        if receiver.try_recv().is_ok() {
            break Ok(());
        }
        if Instant::now() >= deadline {
            break Err("host-run holder timed out".into());
        }
        if let Err(error) = session.service() {
            break Err(error.to_string());
        }
        thread::sleep(Duration::from_millis(3));
    };
    session.close();
    println!("CLOSED");
    result.map_err(Into::into)
}
