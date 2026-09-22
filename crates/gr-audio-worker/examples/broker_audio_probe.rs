//! Ordinary-user installed-broker smoke test. No physical audio is opened.
use gr_privileged_broker::{
    audio_client::Client, audio_launch::Profile, read_message, write_message,
};
use std::{io, time::Duration};
fn main() -> io::Result<()> {
    for profile in [Profile::DualSense, Profile::DualShock4, Profile::Xbox360] {
        let (mut client, [mut control, playback, microphone]) =
            Client::open(profile, [2, 1, 2, 3, 4, 5])?;
        control.set_read_timeout(Some(Duration::from_secs(2)))?;
        control.set_write_timeout(Some(Duration::from_secs(1)))?;
        write_message(&mut control, 3, &client.generation().to_le_bytes())?;
        let (tag, stats) = read_message(&mut control)?;
        if tag != 3 || stats.len() != 72 {
            return Err(io::Error::other("invalid worker diagnostics"));
        }
        println!(
            "{}: device={} bus={} worker diagnostics received",
            profile.name(),
            client.device(),
            client.bus_id()
        );
        client.close()?;
        client.close()?;
        for mut channel in [control, playback, microphone] {
            use std::io::Read;
            channel.set_read_timeout(Some(Duration::from_secs(2)))?;
            if channel.read(&mut [0])? != 0 {
                return Err(io::Error::other("owned endpoint remained open"));
            }
        }
        println!(
            "{}: idempotent broker closure and endpoint removal passed",
            profile.name()
        );
    }
    Ok(())
}
