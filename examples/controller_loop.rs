//! A standalone application owner: no provider imports or routing policy.
use std::{
    error::Error,
    thread,
    time::{Duration, Instant},
};
use virtualgamepad::{
    CreationOptions, RealizationId, create_dualsense, create_dualshock4, create_switch_pro,
    create_xbox360,
};

fn main() -> Result<(), Box<dyn Error>> {
    let options = CreationOptions::new(RealizationId::LINUX_UHID_USB);
    let mut sony = create_dualsense(options)?;
    let mut ds4 = create_dualshock4(options)?;
    let mut switch = create_switch_pro(options)?;
    let mut xbox = create_xbox360(options)?;
    // The application can retain Sony identity independently of session lifetime.
    let _identity_for_application_storage = sony
        .identity()
        .map(virtualgamepad::DualSenseIdentity::to_bytes);
    let until = Instant::now() + Duration::from_secs(2);
    while Instant::now() < until {
        sony.service(&mut |_| {})?;
        ds4.service(&mut |_| {})?;
        switch.service(&mut |_| {})?;
        xbox.service(&mut |_| {})?;
        // A descriptor-aware application may wait on readiness instead. This
        // bounded polling example still honors shorter/immediate deadlines.
        let delay = [
            sony.next_service_in(),
            ds4.next_service_in(),
            switch.next_service_in(),
            xbox.next_service_in(),
        ]
        .into_iter()
        .flatten()
        .fold(Duration::from_millis(4), Duration::min);
        thread::sleep(delay);
    }
    // Independent removal does not change the remaining controllers' ownership.
    xbox.neutralize()?;
    xbox.commit()?;
    xbox.close();
    sony.service(&mut |_| {})?;
    ds4.service(&mut |_| {})?;
    switch.service(&mut |_| {})?;
    sony.close();
    ds4.close();
    switch.close();
    for diagnostics in [
        sony.diagnostics(),
        ds4.diagnostics(),
        switch.diagnostics(),
        xbox.diagnostics(),
    ] {
        if let Some(error) = diagnostics.last_error() {
            eprintln!("{error}");
        }
    }
    Ok(())
}
