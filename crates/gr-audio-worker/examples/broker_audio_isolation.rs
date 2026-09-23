//! Installed-broker isolation checks with synthetic clients, no physical devices.
use gr_privileged_broker::{
    audio_client::Client, audio_launch::Profile, read_message, write_message,
};
use std::{
    io::{self, Read},
    os::unix::net::UnixStream,
    time::{Duration, Instant},
};
fn diagnostics(client: &Client, control: &mut UnixStream) -> io::Result<()> {
    control.set_read_timeout(Some(Duration::from_secs(2)))?;
    write_message(control, 3, &client.generation().to_le_bytes())?;
    let (tag, body) = read_message(control)?;
    if tag != 3 || body.len() != 72 {
        return Err(io::Error::other("invalid worker diagnostics"));
    }
    Ok(())
}
fn gone(bus: &str) -> io::Result<()> {
    let path = format!("/sys/bus/usb/devices/{bus}");
    let deadline = Instant::now() + Duration::from_secs(3);
    while std::path::Path::new(&path).exists() {
        if Instant::now() >= deadline {
            return Err(io::Error::other("owned USB device survived cleanup"));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}
fn main() -> io::Result<()> {
    let mut sessions = Vec::new();
    for profile in [Profile::DualSense, Profile::DualSense, Profile::Xbox360] {
        sessions.push(Client::open(profile, [2, 1, 2, 3, 4, 5])?);
    }
    for (client, channels) in &mut sessions {
        diagnostics(client, &mut channels[0])?;
    }
    match Client::open(Profile::DualShock4, [2, 1, 2, 3, 4, 5]) {
        Err(error)
            if error
                .to_string()
                .contains("no free administrator-allowlisted VHCI port") => {}
        Err(error) => return Err(error),
        Ok(_) => return Err(io::Error::other("expected three-port installation quota")),
    }
    let (mut removed, channels) = sessions.remove(1);
    removed.close()?;
    gone(removed.bus_id())?;
    drop(channels);
    for (client, channels) in &mut sessions {
        diagnostics(client, &mut channels[0])?;
    }
    let (recreated, channels) = Client::open(Profile::DualShock4, [2, 1, 2, 3, 4, 5])?;
    let bus = recreated.bus_id().to_string();
    drop(recreated); // Simulated application death: broker connection disappears.
    for mut channel in channels {
        channel.set_read_timeout(Some(Duration::from_secs(3)))?;
        if channel.read(&mut [0])? != 0 {
            return Err(io::Error::other("client death retained endpoint"));
        }
    }
    gone(&bus)?;
    for (mut client, mut channels) in sessions {
        diagnostics(&client, &mut channels[0])?;
        client.close()?;
        gone(client.bus_id())?;
    }
    // Invalid generation causes worker death. Keep the failed client's broker
    // connection alive: it must not retain admission or an allowlisted port.
    let (client, mut channels) = Client::open(Profile::Xbox360, [2, 1, 2, 3, 4, 5])?;
    write_message(&mut channels[0], 3, &0_u64.to_le_bytes())?;
    for channel in &mut channels {
        channel.set_read_timeout(Some(Duration::from_secs(3)))?;
        if channel.read(&mut [0])? != 0 {
            return Err(io::Error::other("worker death retained endpoint"));
        }
    }
    gone(client.bus_id())?;
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut replacements = Vec::new();
    while replacements.len() != 3 {
        match Client::open(Profile::DualShock4, [2, 1, 2, 3, 4, 5]) {
            Ok(session) => replacements.push(session),
            Err(error)
                if error
                    .to_string()
                    .contains("no free administrator-allowlisted VHCI port")
                    && Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(error) => return Err(error),
        }
    }
    for (mut replacement, mut channels) in replacements {
        diagnostics(&replacement, &mut channels[0])?;
        replacement.close()?;
        gone(replacement.bus_id())?;
    }
    drop(client);
    println!(
        "PASS: duplicate/mixed sessions, port exhaustion, independent middle removal, recreation, client death and idle-client worker-death resource recovery"
    );
    Ok(())
}
