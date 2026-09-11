#![forbid(unsafe_code)]

#[cfg(target_os = "linux")]
fn main() -> Result<(), eframe::Error> {
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("virtualgamepad Demo GUI")
            .with_app_id("virtualgamepad-demo")
            .with_decorations(true)
            .with_minimize_button(true)
            .with_maximize_button(true)
            .with_close_button(true),
        ..Default::default()
    };
    eframe::run_native(
        "virtualgamepad Demo GUI",
        native_options,
        Box::new(|_| Ok(Box::new(gui::App::default()))),
    )
}
#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("virtualgamepad demo currently supports Linux only");
}

#[cfg(target_os = "linux")]
mod gui;
