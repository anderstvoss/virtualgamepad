//! Explicit live GUI soak: neutral owned UHID devices; no audio or touch injection.
#![forbid(unsafe_code)]
#[cfg(target_os = "linux")]
mod gui;

#[cfg(target_os = "linux")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::time::{Duration, Instant};
    struct Soak {
        app: gui::App,
        started: Instant,
        duration: Duration,
        next_sample: Duration,
    }
    impl eframe::App for Soak {
        fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
            self.app.update(ctx, frame);
            let elapsed = self.started.elapsed();
            if elapsed >= self.next_sample {
                self.app
                    .validation_soak_cycle()
                    .expect("owned worker cycle");
                let status = std::fs::read_to_string("/proc/self/status").unwrap();
                let rss = status
                    .lines()
                    .find(|line| line.starts_with("VmRSS:"))
                    .unwrap();
                let fds = std::fs::read_dir("/proc/self/fd").unwrap().count();
                let threads = std::fs::read_dir("/proc/self/task").unwrap().count();
                println!(
                    "gui_soak seconds={} rss={rss:?} fds={fds} threads={threads}",
                    elapsed.as_secs()
                );
                self.next_sample = elapsed + Duration::from_secs(60);
            }
            if elapsed >= self.duration {
                println!("gui_soak completed seconds={}", elapsed.as_secs());
                ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Close);
            }
        }
    }
    let seconds: u64 = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "7200".into())
        .parse()?;
    if !(60..=7200).contains(&seconds) {
        return Err("duration must be 60..7200 seconds".into());
    }
    let app = gui::App::validation_soak()?;
    eframe::run_native(
        "Virtualgamepad isolated GUI soak",
        eframe::NativeOptions::default(),
        Box::new(move |_| {
            Ok(Box::new(Soak {
                app,
                started: Instant::now(),
                duration: Duration::from_secs(seconds),
                next_sample: Duration::from_secs(60),
            }))
        }),
    )?;
    Ok(())
}
#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("GUI soak requires Linux, a display and prepared UHID access");
}
