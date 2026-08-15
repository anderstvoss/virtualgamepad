#![forbid(unsafe_code)]

#[cfg(target_os = "linux")]
fn main() -> Result<(), eframe::Error> {
    eframe::run_native(
        "virtualgamepad",
        eframe::NativeOptions::default(),
        Box::new(|_| Ok(Box::new(gui::App::default()))),
    )
}
#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("virtualgamepad demo currently supports Linux only");
}

#[cfg(target_os = "linux")]
mod gui;
