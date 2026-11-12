//! Owner-observed Steam check: one product controller, scripted inputs, retained outputs.
use std::{
    error::Error,
    io::{self, BufRead},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
use virtualgamepad::{
    CreationOptions, RealizationId, XboxSeriesButton as B, XboxSeriesOptions, XboxSeriesState,
    create_xbox_series,
};

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

fn sample(step: usize) -> XboxSeriesState {
    let mut state = XboxSeriesState::default();
    match step {
        0..32 => state.set_button(BUTTONS[step / 2], step % 2 == 0),
        32..40 => {
            let axis = (step - 32) / 2;
            state.sticks[axis / 2][axis % 2] = if step % 2 == 0 { i16::MIN } else { i16::MAX };
        }
        40..44 => {
            let mut triggers = [0; 2];
            triggers[(step - 40) / 2] = if step % 2 == 0 { 1023 } else { 0 };
            state
                .set_triggers(triggers[0], triggers[1])
                .expect("bounded trigger sample");
        }
        _ => {}
    }
    state
}

fn main() -> Result<(), Box<dyn Error>> {
    let target = match std::env::args().nth(1).as_deref() {
        Some("uinput") => RealizationId::LINUX_UINPUT,
        Some("bluetooth") => RealizationId::LINUX_UHID_BLUETOOTH,
        _ => return Err("expected uinput or bluetooth".into()),
    };
    let mut pad = create_xbox_series(CreationOptions::new(target), XboxSeriesOptions::default())?;
    let (send, commands) = mpsc::channel();
    thread::spawn(move || {
        for line in io::stdin().lock().lines().map_while(Result::ok) {
            if send.send(line).is_err() {
                return;
            }
        }
        let _ = send.send("quit".into());
    });
    println!("Ready: {target:?}; commands: sweep, neutral, quit");
    let mut start = Instant::now();
    let mut sweeping = false;
    let mut previous = None;
    let mut revision = 0;
    'running: loop {
        for command in commands.try_iter() {
            match command.trim() {
                "quit" => break 'running,
                "sweep" => {
                    sweeping = true;
                    start = Instant::now();
                    previous = None;
                }
                "neutral" => {
                    sweeping = false;
                    pad.neutralize()?;
                    pad.commit()?;
                }
                _ => println!("Commands: sweep, neutral, quit"),
            }
        }
        pad.service(&mut |_| {})?;
        if sweeping {
            let step = usize::try_from((start.elapsed().as_millis() / 250) % 45)?;
            if previous != Some(step) {
                pad.set_inputs(sample(step))?;
                pad.commit()?;
                previous = Some(step);
                println!("Input step {step}/44");
            }
        }
        let outputs = pad.host_outputs()?;
        if outputs.revision != revision {
            println!("Retained: {outputs:?}");
            revision = outputs.revision;
        }
        thread::sleep(Duration::from_millis(2));
    }
    pad.neutralize()?;
    pad.commit()?;
    pad.close();
    pad.close();
    println!("Closed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn manual_cycle_covers_buttons_and_native_axis_endpoints_then_neutral() {
        for (index, button) in BUTTONS.into_iter().enumerate() {
            assert!(sample(index * 2).pressed(button));
            assert!(!sample(index * 2 + 1).pressed(button));
        }
        for axis in 0..4 {
            assert_eq!(sample(32 + axis * 2).sticks[axis / 2][axis % 2], i16::MIN);
            assert_eq!(sample(33 + axis * 2).sticks[axis / 2][axis % 2], i16::MAX);
        }
        assert_eq!(sample(40).triggers()[0].raw(), 1023);
        assert_eq!(sample(42).triggers()[1].raw(), 1023);
        assert_eq!(sample(44), XboxSeriesState::default());
    }
}
