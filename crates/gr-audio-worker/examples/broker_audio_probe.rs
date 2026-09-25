//! Ordinary-user installed-broker smoke test. No physical audio is opened.
#[cfg(target_os = "linux")]
mod linux {
    use gr_privileged_broker::{audio_client::Client, audio_launch::Profile};
    use std::{io, time::Duration};
    pub fn run() -> io::Result<()> {
        for profile in [Profile::DualSense, Profile::DualShock4, Profile::Xbox360] {
            let (mut client, [control, playback, microphone]) =
                Client::open(profile, [2, 1, 2, 3, 4, 5])?;
            control.set_read_timeout(Some(Duration::from_secs(2)))?;
            control.set_write_timeout(Some(Duration::from_secs(1)))?;
            let (family, state) = match profile {
                Profile::DualSense => (
                    1,
                    gr_curated_controllers::usb_personality::state::NativeState::DualSense(
                        gr_curated_controllers::DualSenseState::default(),
                    ),
                ),
                Profile::DualShock4 => (
                    2,
                    gr_curated_controllers::usb_personality::state::NativeState::DualShock4(
                        gr_curated_controllers::DualShock4State::default(),
                    ),
                ),
                Profile::Xbox360 => (
                    3,
                    gr_curated_controllers::usb_personality::state::NativeState::Xbox360(
                        gr_curated_controllers::Xbox360State::default(),
                    ),
                ),
            };
            let mut commands = gr_audio_worker::client::Control::new(
                control.try_clone()?,
                client.generation(),
                family,
            )?;
            commands.update(&state)?;
            commands.update(&state)?;
            commands.diagnostics()?;
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
}

#[cfg(target_os = "linux")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    linux::run()?;
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("This audio probe requires Linux.");
}
