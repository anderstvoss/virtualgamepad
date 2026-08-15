use eframe::egui::{self, Button, Color32, Pos2, Sense, Stroke, Vec2};
use gr_privileged_broker::BrokerClient;
use std::{
    sync::{Arc, Mutex, mpsc},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use virtualgamepad::ControllerSurfaceInfo;
use virtualgamepad::{
    BatteryLevel, BatteryState, CreationOptions, DigitalControlUpdate, DpadDirection,
    DualSenseAxis, DualSenseControl, DualSenseController, DualSenseHidOutput, DualSenseOutputEvent,
    DualSenseTouchContact, DualSenseTrigger, DualShock4Axis, DualShock4Control,
    DualShock4Controller, DualShock4HidOutput, DualShock4MotionSample, DualShock4TouchContact,
    DualShock4TouchSlot, DualShock4Trigger, FaceButton, MotionSample, RealizationSessionId,
    RealizationTarget, SwitchProAxis, SwitchProControl, SwitchProController, SwitchProMotionSample,
    TouchSlot, Xbox360Axis, Xbox360Control, Xbox360Controller, Xbox360OutputEvent, Xbox360Trigger,
    create_dualsense, create_dualshock4, create_switch_pro, create_xbox360,
};

const OUTPUT_LOG_LIMIT: usize = 200;
const DUALSENSE_MOTION_INTERVAL: Duration = Duration::from_millis(4);
const IDLE_REPAINT_INTERVAL: Duration = Duration::from_millis(50);

fn dualsense_motion_target(target: RealizationTarget) -> bool {
    matches!(
        target,
        RealizationTarget::Uhid | RealizationTarget::DummyHcd
    )
}

fn dualsense_motion_target_label(target: RealizationTarget) -> &'static str {
    if target == RealizationTarget::Uhid {
        "UHID motion report"
    } else {
        "DummyHcd USB motion report"
    }
}

fn dummy_hcd_broker_status() -> Result<(), String> {
    BrokerClient::connect()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn repaint_interval(controller_count: usize) -> Duration {
    if controller_count == 0 {
        IDLE_REPAINT_INTERVAL
    } else {
        DUALSENSE_MOTION_INTERVAL
    }
}

const fn motion_worker_interval() -> Duration {
    DUALSENSE_MOTION_INTERVAL
}

fn controller_tab_indices(controller_count: usize) -> std::ops::Range<usize> {
    0..controller_count
}

fn selection_after_removal(remaining_count: usize, removed_index: usize) -> Option<usize> {
    if remaining_count == 0 {
        None
    } else {
        Some(removed_index.min(remaining_count - 1))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ControllerLifecycleStatus {
    Created { name: String },
    CreationFailed { error: String },
    ClosedAfterFailure { name: String, error: String },
}

fn status_after_runtime_failure(name: &str, error: String) -> ControllerLifecycleStatus {
    ControllerLifecycleStatus::ClosedAfterFailure {
        name: name.into(),
        error,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Xbox360,
    DualSense,
    DualShock4,
    SwitchPro,
}
impl Kind {
    const ALL: [Self; 4] = [
        Self::Xbox360,
        Self::DualSense,
        Self::DualShock4,
        Self::SwitchPro,
    ];
    const fn label(self) -> &'static str {
        match self {
            Self::Xbox360 => "Xbox 360",
            Self::DualSense => "DualSense",
            Self::DualShock4 => "DualShock 4",
            Self::SwitchPro => "Switch Pro Controller",
        }
    }
}
enum Controller {
    Xbox(Xbox360Controller),
    DualSense(DualSenseController),
    DualShock4(DualShock4Controller),
    SwitchPro(SwitchProController),
}

struct NamedController {
    kind: Kind,
    name: String,
    controller: Arc<Mutex<Controller>>,
    indicators: ReverseIndicators,
    motion_worker: Option<MotionWorker>,
}

struct MotionWorker {
    stop: mpsc::Sender<()>,
    failure: mpsc::Receiver<String>,
    handle: JoinHandle<()>,
}

impl MotionWorker {
    fn stop(self) {
        let _ = self.stop.send(());
        let _ = self.handle.join();
    }
}

fn start_motion_worker(controller: &Arc<Mutex<Controller>>) -> Option<MotionWorker> {
    if !controller
        .lock()
        .expect("controller mutex is not poisoned during creation")
        .needs_motion_refresh()
    {
        return None;
    }
    let (stop_sender, stop_receiver) = mpsc::channel();
    let (failure_sender, failure_receiver) = mpsc::channel();
    let controller = Arc::clone(controller);
    let handle = thread::spawn(move || {
        loop {
            match stop_receiver.recv_timeout(motion_worker_interval()) {
                Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
            let result = controller
                .lock()
                .map_err(|_| "controller mutex poisoned in motion worker".to_owned())
                .and_then(|mut controller| controller.refresh_motion());
            if let Err(error) = result {
                let _ = failure_sender.send(error);
                break;
            }
        }
    });
    Some(MotionWorker {
        stop: stop_sender,
        failure: failure_receiver,
        handle,
    })
}

#[derive(Default)]
struct ReverseIndicators {
    led: Option<[u8; 3]>,
    mute_led: Option<bool>,
    rumble_until: Option<Instant>,
    rumble_active: bool,
    rumble_started: Option<Instant>,
}
impl ReverseIndicators {
    fn rumble_pulse(&mut self) {
        self.rumble_until = Some(Instant::now() + Duration::from_millis(750));
    }
    fn set_rumble(&mut self, active: bool) {
        if active && !self.rumble_active {
            self.rumble_started = Some(Instant::now());
        }
        self.rumble_active = active;
        if active {
            self.rumble_until = None;
        }
    }
    fn apply_dualsense_usb_output(
        &mut self,
        right_motor: Option<u8>,
        left_motor: Option<u8>,
        lightbar_rgb: Option<[u8; 3]>,
        mute_button_led: Option<bool>,
    ) {
        self.set_rumble(
            right_motor.is_some_and(|motor| motor != 0)
                || left_motor.is_some_and(|motor| motor != 0),
        );
        if let Some(lightbar_rgb) = lightbar_rgb {
            self.led = Some(lightbar_rgb);
        }
        if let Some(mute_button_led) = mute_button_led {
            self.mute_led = Some(mute_button_led);
        }
    }
}
impl Controller {
    fn needs_motion_refresh(&self) -> bool {
        matches!(self, Self::DualSense(controller) if dualsense_motion_target(controller.surface().common().target))
            || matches!(self, Self::DualShock4(controller) if controller.surface().common().target == RealizationTarget::Uhid)
            || matches!(self, Self::SwitchPro(controller) if controller.surface().common().target == RealizationTarget::Uhid)
    }

    fn refresh_motion(&mut self) -> Result<(), String> {
        match self {
            Self::DualSense(controller)
                if dualsense_motion_target(controller.surface().common().target) =>
            {
                controller
                    .set_motion(controller.state().motion())
                    .map_err(|error| error.to_string())?;
                controller.commit().map_err(|error| error.to_string())
            }
            Self::DualShock4(controller)
                if controller.surface().common().target == RealizationTarget::Uhid =>
            {
                controller
                    .set_motion(controller.state().motion())
                    .map_err(|error| error.to_string())?;
                controller.commit().map_err(|error| error.to_string())
            }
            Self::SwitchPro(controller)
                if controller.surface().common().target == RealizationTarget::Uhid =>
            {
                controller
                    .refresh_motion()
                    .map_err(|error| error.to_string())
            }
            _ => Ok(()),
        }
    }

    fn commit(&mut self) -> Result<(), String> {
        let result = match self {
            Self::Xbox(controller) => controller.commit(),
            Self::DualSense(controller) => controller.commit(),
            Self::DualShock4(controller) => controller.commit(),
            Self::SwitchPro(controller) => controller.commit(),
        };
        result.map_err(|error| error.to_string())
    }
    fn close(&mut self) {
        match self {
            Self::Xbox(controller) => controller.close(),
            Self::DualSense(controller) => controller.close(),
            Self::DualShock4(controller) => controller.close(),
            Self::SwitchPro(controller) => controller.close(),
        }
    }
    fn is_dirty(&self) -> bool {
        match self {
            Self::Xbox(controller) => controller.is_dirty(),
            Self::DualSense(controller) => controller.is_dirty(),
            Self::DualShock4(controller) => controller.is_dirty(),
            Self::SwitchPro(controller) => controller.is_dirty(),
        }
    }
    #[allow(clippy::too_many_lines)] // Acknowledgements must stay adjacent to typed decoding.
    fn poll_output(
        &mut self,
        log: &mut Vec<String>,
        indicators: &mut ReverseIndicators,
    ) -> Result<(), String> {
        let result: Result<(), String> = match self {
            Self::Xbox(controller) => {
                let mut replies = Vec::new();
                controller
                    .poll_output(&mut |event| {
                        match event {
                            Xbox360OutputEvent::ForceFeedbackUpload { request_id, .. } => {
                                indicators.rumble_pulse();
                                replies.push((request_id, true));
                            }
                            Xbox360OutputEvent::ForceFeedbackErase { request_id, .. } => {
                                replies.push((request_id, false));
                            }
                            _ => {}
                        }
                        log.push(format!("Xbox 360: {event:?}"));
                    })
                    .map_err(|error| error.to_string())?;
                for (request_id, upload) in replies {
                    if upload {
                        controller
                            .reply_force_feedback_upload(request_id, 0)
                            .map_err(|error| error.to_string())?;
                    } else {
                        controller
                            .reply_force_feedback_erase(request_id, 0)
                            .map_err(|error| error.to_string())?;
                    }
                }
                Ok(())
            }
            Self::DualSense(controller) => {
                let mut replies = Vec::new();
                controller
                    .poll_output(&mut |event| {
                        match &event {
                            DualSenseOutputEvent::ConventionalForceFeedbackUpload {
                                request_id,
                                ..
                            } => {
                                indicators.rumble_pulse();
                                replies.push((*request_id, true));
                            }
                            DualSenseOutputEvent::ConventionalForceFeedbackErase {
                                request_id,
                                ..
                            } => {
                                replies.push((*request_id, false));
                            }
                            DualSenseOutputEvent::HidOutput(DualSenseHidOutput::UsbOutput {
                                right_motor,
                                left_motor,
                                lightbar_rgb,
                                mute_button_led,
                                ..
                            }) => {
                                indicators.apply_dualsense_usb_output(
                                    *right_motor,
                                    *left_motor,
                                    *lightbar_rgb,
                                    *mute_button_led,
                                );
                            }
                            _ => {}
                        }
                        log.push(format!("DualSense: {event:?}"));
                    })
                    .map_err(|error| error.to_string())?;
                for (request_id, upload) in replies {
                    if upload {
                        controller
                            .reply_force_feedback_upload(request_id, 0)
                            .map_err(|error| error.to_string())?;
                    } else {
                        controller
                            .reply_force_feedback_erase(request_id, 0)
                            .map_err(|error| error.to_string())?;
                    }
                }
                Ok(())
            }
            Self::DualShock4(controller) => controller
                .poll_output(&mut |event| {
                    if let virtualgamepad::DualShock4OutputEvent::HidOutput(
                        DualShock4HidOutput::UsbOutput {
                            right_motor,
                            left_motor,
                            ..
                        },
                    ) = &event
                    {
                        indicators.set_rumble(*right_motor != 0 || *left_motor != 0);
                    }
                    log.push(format!("DualShock 4: {event:?}"));
                })
                .map_err(|error| error.to_string()),
            Self::SwitchPro(controller) => controller
                .poll_output(&mut |event| log.push(format!("Switch Pro: {event:?}")))
                .map_err(|error| error.to_string()),
        };
        result
    }
    fn draw(&mut self, ui: &mut egui::Ui) {
        if matches!(self, Self::Xbox(_) | Self::DualSense(_)) {
            let battery = self.battery();
            ui.group(|ui| {
                ui.label("Battery emulation");
                let mut exposed = battery.is_exposed();
                if ui.checkbox(&mut exposed, "Expose battery").changed() {
                    let _ = self.set_battery_exposed(exposed);
                }
                if exposed {
                    let mut level = battery.level().percent();
                    if ui
                        .add(egui::Slider::new(&mut level, 0..=100).text("Battery level (%)"))
                        .changed()
                    {
                        if let Ok(level) = BatteryLevel::new(level) {
                            let _ = self.set_battery_level(level);
                        }
                    }
                }
            });
        }
        match self {
            Self::Xbox(controller) => draw_xbox(ui, controller),
            Self::DualSense(controller) => draw_dualsense(ui, controller),
            Self::DualShock4(controller) => draw_dualshock4(ui, controller),
            Self::SwitchPro(controller) => draw_switch_pro(ui, controller),
        }
    }
    fn battery(&self) -> BatteryState {
        match self {
            Self::Xbox(controller) => controller.state().battery(),
            Self::DualSense(controller) => controller.state().battery(),
            Self::DualShock4(_) | Self::SwitchPro(_) => BatteryState::default(),
        }
    }
    fn set_battery_exposed(&mut self, exposed: bool) -> Result<(), String> {
        match self {
            Self::Xbox(controller) => controller.set_battery_exposed(exposed),
            Self::DualSense(controller) => controller.set_battery_exposed(exposed),
            Self::DualShock4(_) | Self::SwitchPro(_) => Ok(()),
        }
        .map_err(|error| error.to_string())
    }
    fn set_battery_level(&mut self, level: BatteryLevel) -> Result<(), String> {
        match self {
            Self::Xbox(controller) => controller.set_battery_level(level),
            Self::DualSense(controller) => controller.set_battery_level(level),
            Self::DualShock4(_) | Self::SwitchPro(_) => Ok(()),
        }
        .map_err(|error| error.to_string())
    }
}
pub struct App {
    kind: Kind,
    target: RealizationTarget,
    name_draft: String,
    next_session: u64,
    controllers: Vec<NamedController>,
    selected_controller: Option<usize>,
    output_log: Vec<String>,
    lifecycle_status: Option<ControllerLifecycleStatus>,
}
impl Default for App {
    fn default() -> Self {
        Self {
            kind: Kind::Xbox360,
            target: RealizationTarget::Evdev,
            name_draft: String::new(),
            next_session: 1,
            controllers: vec![],
            selected_controller: None,
            output_log: vec![],
            lifecycle_status: None,
        }
    }
}
impl App {
    fn next_default_name(&self) -> String {
        let number = self
            .controllers
            .iter()
            .filter(|controller| controller.kind == self.kind)
            .count();
        format!("{} {number}", self.kind.label())
    }

    fn create(&mut self) {
        let options = CreationOptions {
            target: self.target,
            session: RealizationSessionId(self.next_session),
        };
        let result = match self.kind {
            Kind::Xbox360 => create_xbox360(options).map(Controller::Xbox),
            Kind::DualSense => create_dualsense(options).map(Controller::DualSense),
            Kind::DualShock4 => create_dualshock4(options).map(Controller::DualShock4),
            Kind::SwitchPro => create_switch_pro(options).map(Controller::SwitchPro),
        };
        match result {
            Ok(controller) => {
                let name = if self.name_draft.trim().is_empty() {
                    self.next_default_name()
                } else {
                    self.name_draft.trim().to_owned()
                };
                let controller = Arc::new(Mutex::new(controller));
                let motion_worker = start_motion_worker(&controller);
                self.controllers.push(NamedController {
                    kind: self.kind,
                    name: name.clone(),
                    controller,
                    indicators: ReverseIndicators::default(),
                    motion_worker,
                });
                self.selected_controller = Some(self.controllers.len() - 1);
                self.name_draft.clear();
                self.next_session += 1;
                self.lifecycle_status = Some(ControllerLifecycleStatus::Created { name });
            }
            Err(error) => {
                let error = error.to_string();
                self.lifecycle_status = Some(ControllerLifecycleStatus::CreationFailed { error });
            }
        }
    }

    fn remove_controller(&mut self, index: usize) {
        if index >= self.controllers.len() {
            return;
        }
        let mut removed = self.controllers.remove(index);
        if let Some(worker) = removed.motion_worker.take() {
            worker.stop();
        }
        if let Ok(mut controller) = removed.controller.lock() {
            controller.close();
        }
        self.selected_controller = selection_after_removal(self.controllers.len(), index);
    }

    fn close_failed_controller(&mut self, index: usize, error: String) {
        let name = self.controllers[index].name.clone();
        self.lifecycle_status = Some(status_after_runtime_failure(&name, error));
        self.remove_controller(index);
    }
}
impl Drop for App {
    fn drop(&mut self) {
        for named in &mut self.controllers {
            if let Some(worker) = named.motion_worker.take() {
                worker.stop();
            }
            if let Ok(mut controller) = named.controller.lock() {
                controller.close();
            }
        }
    }
}
impl eframe::App for App {
    #[allow(clippy::too_many_lines)] // Coordinates the independent demo panels.
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        let mut remove = None;
        let mut failed_controller = None;
        for (index, named) in self.controllers.iter_mut().enumerate() {
            if let Some(worker) = &named.motion_worker {
                if let Ok(error) = worker.failure.try_recv() {
                    failed_controller = Some((index, error));
                    break;
                }
            }
            let result = named
                .controller
                .lock()
                .map_err(|_| "controller mutex poisoned while polling output".to_owned())
                .and_then(|mut controller| {
                    controller.poll_output(&mut self.output_log, &mut named.indicators)
                });
            if let Err(error) = result {
                failed_controller = Some((index, error));
                break;
            }
        }
        if self.output_log.len() > OUTPUT_LOG_LIMIT {
            let excess = self.output_log.len() - OUTPUT_LOG_LIMIT;
            self.output_log.drain(..excess);
        }
        ctx.request_repaint_after(repaint_interval(self.controllers.len()));
        egui::SidePanel::left("create").show(ctx, |ui| {
            ui.heading("Create controller");
            egui::ComboBox::from_label("Type")
                .selected_text(self.kind.label())
                .show_ui(ui, |ui| {
                    for kind in Kind::ALL {
                        ui.selectable_value(&mut self.kind, kind, kind.label());
                    }
                });
            egui::ComboBox::from_label("Target")
                .selected_text(target_label(self.target))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.target,
                        RealizationTarget::Evdev,
                        target_label(RealizationTarget::Evdev),
                    );
                    ui.selectable_value(
                        &mut self.target,
                        RealizationTarget::Uhid,
                        target_label(RealizationTarget::Uhid),
                    );
                    ui.selectable_value(
                        &mut self.target,
                        RealizationTarget::DummyHcd,
                        target_label(RealizationTarget::DummyHcd),
                    );
                });
            let default_name = self.next_default_name();
            ui.add(
                egui::TextEdit::singleline(&mut self.name_draft)
                    .hint_text(default_name)
                    .desired_width(f32::INFINITY),
            )
            .on_hover_text("Optional name. Leave empty for the automatic controller name.");
            ui.small("UHID requires /dev/uhid access. DummyHcd requires the administrator-installed broker service.");
            if self.target == RealizationTarget::DummyHcd {
                match dummy_hcd_broker_status() {
                    Ok(()) => {
                        ui.colored_label(
                            Color32::GREEN,
                            "DummyHcd broker socket is reachable. Create DualSense to attach a USB device.",
                        );
                    }
                    Err(error) => {
                        ui.colored_label(
                            Color32::RED,
                            format!("DummyHcd broker unavailable: {error}"),
                        );
                    }
                }
                ui.small(
                    "Test flow: select DualSense, create it, then exercise buttons, touch, motion, and host-output indicators.",
                );
            }
            if ui.button("Create").clicked() {
                self.create();
            }
            ui.separator();
            ui.label("Controllers");
            egui::ScrollArea::vertical()
                .max_height(260.0)
                .show(ui, |ui| {
                    for index in controller_tab_indices(self.controllers.len()) {
                        let controller = &self.controllers[index];
                        ui.horizontal(|ui| {
                            if ui
                                .selectable_label(
                                    self.selected_controller == Some(index),
                                    &controller.name,
                                )
                                .clicked()
                            {
                                self.selected_controller = Some(index);
                            }
                            if ui
                                .small_button("×")
                                .on_hover_text("Remove controller")
                                .clicked()
                            {
                                remove = Some(index);
                            }
                        });
                    }
                });
            if let Some(status) = &self.lifecycle_status {
                match status {
                    ControllerLifecycleStatus::Created { name } => {
                        ui.colored_label(Color32::GREEN, format!("Created {name}."));
                    }
                    ControllerLifecycleStatus::CreationFailed { error } => {
                        ui.colored_label(Color32::RED, format!("Creation failed: {error}"));
                    }
                    ControllerLifecycleStatus::ClosedAfterFailure { name, error } => {
                        ui.colored_label(
                            Color32::RED,
                            format!("{name} closed after provider failure: {error}"),
                        );
                    }
                }
            }
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.heading("Live controllers");
                    if let Some(index) = self
                        .selected_controller
                        .filter(|index| *index < self.controllers.len())
                    {
                        let named = &mut self.controllers[index];
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.label("Controller name:");
                                ui.text_edit_singleline(&mut named.name);
                            });
                            draw_reverse_indicators(ui, &named.indicators);
                            let result = named
                                .controller
                                .lock()
                                .map_err(|_| "controller mutex poisoned while drawing".to_owned())
                                .and_then(|mut controller| {
                                    controller.draw(ui);
                                    if controller.is_dirty() {
                                        controller.commit()?;
                                    }
                                    Ok(())
                                });
                            if let Err(error) = result {
                                failed_controller = Some((index, error));
                            }
                            ui.small("Input changes are sent automatically.");
                        });
                    } else {
                        ui.small("Create a controller, then select its tab.");
                    }
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.heading("Live typed reverse output");
                        if ui.button("Clear").clicked() {
                            self.output_log.clear();
                        }
                    });
                    ui.small("Polling every 50 ms while the demo is open.");
                    if self.output_log.is_empty() {
                        ui.small("No reverse output received.");
                    }
                    for entry in self.output_log.iter().rev().take(20) {
                        ui.monospace(entry);
                    }
                });
        });
        if let Some((index, error)) = failed_controller {
            self.close_failed_controller(index, error);
        } else if let Some(index) = remove {
            self.remove_controller(index);
        }
    }
}

const fn target_label(target: RealizationTarget) -> &'static str {
    match target {
        RealizationTarget::Evdev => "Evdev / uinput",
        RealizationTarget::Uhid => "HID / UHID",
        RealizationTarget::DummyHcd => "USB / dummy_hcd",
        _ => "Unknown target",
    }
}

fn draw_reverse_indicators(ui: &mut egui::Ui, indicators: &ReverseIndicators) {
    ui.horizontal(|ui| {
        ui.label("Reverse effects:");
        let led = indicators.led.unwrap_or([30, 30, 30]);
        let (led_rect, _) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::hover());
        ui.painter()
            .rect_filled(led_rect, 2.0, Color32::from_rgb(led[0], led[1], led[2]));
        ui.label("LED");

        let (mute_rect, _) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::hover());
        ui.painter().circle_filled(
            mute_rect.center(),
            7.0,
            if indicators.mute_led == Some(true) {
                Color32::from_rgb(255, 130, 40)
            } else {
                Color32::DARK_GRAY
            },
        );
        ui.label("Mute LED");

        let remaining = indicators
            .rumble_until
            .map(|until| until.saturating_duration_since(Instant::now()))
            .unwrap_or_default();
        let active = indicators.rumble_active || !remaining.is_zero();
        let (rumble_rect, _) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::hover());
        let phase = if indicators.rumble_active {
            indicators
                .rumble_started
                .map(|started| started.elapsed().as_secs_f32() * 8.0)
                .unwrap_or_default()
                .sin()
                .abs()
        } else {
            (remaining.as_secs_f32() * 8.0).sin().abs()
        };
        let radius = if active { 5.0 + phase * 4.0 } else { 5.0 };
        ui.painter().circle_filled(
            rumble_rect.center(),
            radius,
            if active {
                Color32::from_rgb(220, 80, 80)
            } else {
                Color32::DARK_GRAY
            },
        );
        ui.label("Rumble");
    });
}

fn digital_controls(ui: &mut egui::Ui, mut set: impl FnMut(DigitalControlUpdate)) {
    ui.group(|ui| {
        ui.label("Face buttons");
        ui.horizontal_wrapped(|ui| {
            for (label, button) in [
                ("South", FaceButton::South),
                ("East", FaceButton::East),
                ("West", FaceButton::West),
                ("North", FaceButton::North),
            ] {
                hold(ui, label, |pressed| {
                    set(DigitalControlUpdate::FaceButton { button, pressed });
                });
            }
        });
        ui.label("D-pad");
        ui.horizontal_wrapped(|ui| {
            for (label, direction) in [
                ("Up", DpadDirection::Up),
                ("Down", DpadDirection::Down),
                ("Left", DpadDirection::Left),
                ("Right", DpadDirection::Right),
            ] {
                hold(ui, label, |pressed| {
                    set(DigitalControlUpdate::Dpad { direction, pressed });
                });
            }
        });
    });
}

fn hold(ui: &mut egui::Ui, label: &str, mut set: impl FnMut(bool)) {
    let response = ui.add(Button::new(label));
    let held = response.is_pointer_button_down_on();
    let previous = ui
        .data(|data| data.get_temp::<bool>(response.id))
        .unwrap_or(false);
    if held != previous {
        ui.data_mut(|data| data.insert_temp(response.id, held));
        set(held);
    }
}
fn surface(ui: &mut egui::Ui, surface: &dyn ControllerSurfaceInfo) {
    ui.collapsing("Selected target surface", |ui| {
        let surface = surface.common_surface();
        ui.label(format!("Target: {}", surface.target));
        ui.label(format!("Evidence: {:?}", surface.validation_status));
        ui.label(format!(
            "{} axes, {} digital controls, {} output channels",
            surface.axes.len(),
            surface.digital_controls.len(),
            surface.outputs.len()
        ));
        for axis in surface.axes {
            ui.monospace(format!(
                "{}: code {} {}..={} (neutral {})",
                axis.control, axis.event_code, axis.minimum, axis.maximum, axis.neutral
            ));
        }
        for restriction in surface.restrictions {
            ui.small(format!(
                "Unavailable: {} — {}",
                restriction.feature, restriction.reason
            ));
        }
    });
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn axis_pad(ui: &mut egui::Ui, label: &str, x: &mut i16, y: &mut i16) -> bool {
    ui.vertical(|ui| {
        ui.label(label);
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(112.0), Sense::click_and_drag());
        ui.painter().rect_stroke(
            rect,
            2.0,
            Stroke::new(1.0, Color32::GRAY),
            egui::StrokeKind::Inside,
        );
        ui.painter().line_segment(
            [
                Pos2::new(rect.left(), rect.center().y),
                Pos2::new(rect.right(), rect.center().y),
            ],
            Stroke::new(1.0, Color32::DARK_GRAY),
        );
        ui.painter().line_segment(
            [
                Pos2::new(rect.center().x, rect.top()),
                Pos2::new(rect.center().x, rect.bottom()),
            ],
            Stroke::new(1.0, Color32::DARK_GRAY),
        );
        let pointer = Pos2::new(
            rect.center().x + f32::from(*x) / 32768.0 * rect.width() / 2.0,
            rect.center().y + f32::from(*y) / 32768.0 * rect.height() / 2.0,
        );
        ui.painter()
            .circle_filled(pointer, 5.0, Color32::LIGHT_BLUE);
        let mut changed = false;
        if response.is_pointer_button_down_on() {
            if let Some(position) = response.interact_pointer_pos() {
                let next_x = (((position.x - rect.center().x) / (rect.width() / 2.0))
                    .clamp(-1.0, 1.0)
                    * 32767.0) as i16;
                let next_y = (((position.y - rect.center().y) / (rect.height() / 2.0))
                    .clamp(-1.0, 1.0)
                    * 32767.0) as i16;
                changed = *x != next_x || *y != next_y;
                *x = next_x;
                *y = next_y;
            }
        } else if response.drag_stopped() || response.clicked() {
            changed = *x != 0 || *y != 0;
            *x = 0;
            *y = 0;
        }
        ui.monospace(format!("x={x} y={y}"));
        changed
    })
    .inner
}

fn momentary_trigger(ui: &mut egui::Ui, label: &str, value: &mut u8) -> bool {
    let response = ui.add(egui::Slider::new(value, 0..=255).text(label));
    let mut changed = response.changed();
    if response.drag_stopped() || response.clicked() {
        changed |= *value != 0;
        *value = 0;
    }
    changed
}

fn reset_momentary_motion_axis(value: &mut i16, rest: i16) -> bool {
    let changed = *value != rest;
    *value = rest;
    changed
}

fn momentary_motion_axis(ui: &mut egui::Ui, label: &str, value: &mut i16, rest: i16) -> bool {
    let response = ui.add(egui::Slider::new(value, i16::MIN..=i16::MAX).text(label));
    let mut changed = response.changed();
    if response.drag_stopped() {
        changed |= reset_momentary_motion_axis(value, rest);
    }
    changed
}

fn dualsense_axis_to_pad(value: u8) -> i16 {
    i16::try_from((i32::from(value) - 128) * 257).expect("DualSense axis fits signed pad")
}

fn dualsense_axis_from_pad(value: i16) -> u8 {
    u8::try_from((i32::from(value) / 257 + 128).clamp(0, 255))
        .expect("clamped DualSense axis fits u8")
}
fn draw_xbox(ui: &mut egui::Ui, controller: &mut Xbox360Controller) {
    surface(ui, controller.surface());
    digital_controls(ui, |update| {
        let _ = controller.set_digital(update);
    });
    ui.group(|ui| {
        ui.label("Additional buttons");
        ui.horizontal_wrapped(|ui| {
            for (label, control) in [
                ("Back", Xbox360Control::Back),
                ("Start", Xbox360Control::Start),
                ("Guide", Xbox360Control::Guide),
            ] {
                hold(ui, label, |pressed| {
                    let _ = controller.set_native(control, pressed);
                });
            }
        });
    });
    let (left_x, left_y) = controller.state().left_stick();
    let mut x = left_x.raw();
    let mut y = left_y.raw();
    let (right_x, right_y) = controller.state().right_stick();
    let mut right_x = right_x.raw();
    let mut right_y = right_y.raw();
    ui.group(|ui| {
        ui.label("Sticks");
        ui.horizontal_wrapped(|ui| {
            ui.vertical(|ui| {
                if axis_pad(ui, "Xbox left stick", &mut x, &mut y) {
                    let _ = controller.set_left_stick(Xbox360Axis::new(x), Xbox360Axis::new(y));
                }
                hold(ui, "Left stick press", |pressed| {
                    let _ = controller.set_native(Xbox360Control::LeftStickPress, pressed);
                });
            });
            ui.vertical(|ui| {
                if axis_pad(ui, "Xbox right stick", &mut right_x, &mut right_y) {
                    let _ = controller
                        .set_right_stick(Xbox360Axis::new(right_x), Xbox360Axis::new(right_y));
                }
                hold(ui, "Right stick press", |pressed| {
                    let _ = controller.set_native(Xbox360Control::RightStickPress, pressed);
                });
            });
        });
    });
    let (left, right) = controller.state().triggers();
    let mut left = left.raw();
    let mut right = right.raw();
    if momentary_trigger(ui, "Xbox left trigger", &mut left)
        | momentary_trigger(ui, "Xbox right trigger", &mut right)
    {
        let _ = controller.set_triggers(Xbox360Trigger::new(left), Xbox360Trigger::new(right));
    }
    ui.horizontal_wrapped(|ui| {
        hold(ui, "Left shoulder", |pressed| {
            let _ = controller.set_native(Xbox360Control::LeftShoulder, pressed);
        });
        hold(ui, "Right shoulder", |pressed| {
            let _ = controller.set_native(Xbox360Control::RightShoulder, pressed);
        });
    });
}
#[allow(clippy::too_many_lines)] // Keeps the controller-specific test surface together.
fn draw_dualsense(ui: &mut egui::Ui, controller: &mut DualSenseController) {
    surface(ui, controller.surface());
    digital_controls(ui, |update| {
        let _ = controller.set_digital(update);
    });
    ui.group(|ui| {
        ui.label("Additional buttons");
        ui.horizontal_wrapped(|ui| {
            for (label, control) in [
                ("Create", DualSenseControl::Create),
                ("Options", DualSenseControl::Options),
                ("PlayStation", DualSenseControl::PlayStation),
                ("Touchpad click", DualSenseControl::TouchpadClick),
                ("Microphone mute", DualSenseControl::MicrophoneMute),
            ] {
                hold(ui, label, |pressed| {
                    let _ = controller.set_native(control, pressed);
                });
            }
        });
    });
    let (left_x, left_y) = controller.state().left_stick();
    let mut x = dualsense_axis_to_pad(left_x.raw());
    let mut y = dualsense_axis_to_pad(left_y.raw());
    let (right_x, right_y) = controller.state().right_stick();
    let mut right_x = dualsense_axis_to_pad(right_x.raw());
    let mut right_y = dualsense_axis_to_pad(right_y.raw());
    ui.group(|ui| {
        ui.label("Sticks");
        ui.horizontal_wrapped(|ui| {
            ui.vertical(|ui| {
                if axis_pad(ui, "DualSense left stick", &mut x, &mut y) {
                    let _ = controller.set_left_stick(
                        DualSenseAxis::new(dualsense_axis_from_pad(x)),
                        DualSenseAxis::new(dualsense_axis_from_pad(y)),
                    );
                }
                hold(ui, "Left stick press", |pressed| {
                    let _ = controller.set_native(DualSenseControl::LeftStickPress, pressed);
                });
            });
            ui.vertical(|ui| {
                if axis_pad(ui, "DualSense right stick", &mut right_x, &mut right_y) {
                    let _ = controller.set_right_stick(
                        DualSenseAxis::new(dualsense_axis_from_pad(right_x)),
                        DualSenseAxis::new(dualsense_axis_from_pad(right_y)),
                    );
                }
                hold(ui, "Right stick press", |pressed| {
                    let _ = controller.set_native(DualSenseControl::RightStickPress, pressed);
                });
            });
        });
    });
    let (left, right) = controller.state().triggers();
    let mut left = left.raw();
    let mut right = right.raw();
    if momentary_trigger(ui, "DualSense left trigger", &mut left)
        | momentary_trigger(ui, "DualSense right trigger", &mut right)
    {
        let _ = controller.set_triggers(DualSenseTrigger::new(left), DualSenseTrigger::new(right));
    }
    ui.horizontal_wrapped(|ui| {
        hold(ui, "L1", |pressed| {
            let _ = controller.set_native(DualSenseControl::L1, pressed);
        });
        hold(ui, "R1", |pressed| {
            let _ = controller.set_native(DualSenseControl::R1, pressed);
        });
    });
    ui.group(|ui| {
        ui.label("Touchpad");
        draw_touchpad(ui, controller);
        draw_touch_slot(ui, controller, TouchSlot::Second, 1, "Second contact");
    });
    let target = controller.surface().common().target;
    if dualsense_motion_target(target) {
        ui.group(|ui| {
            ui.label(dualsense_motion_target_label(target));
            let diagnostics = controller.provider_diagnostics();
            ui.small(format!(
                "HID reports sent: {}; host requests handled: {}",
                diagnostics.frames_sent, diagnostics.reverse_events_drained
            ));
            let motion = controller.state().motion();
            let mut gyro = motion.gyroscope;
            let mut accelerometer = motion.accelerometer;
            let mut changed = false;
            for (label, value) in ["Gyro X", "Gyro Y", "Gyro Z"].into_iter().zip(&mut gyro) {
                changed |= momentary_motion_axis(ui, label, value, 0);
            }
            for (label, value) in ["Accel X", "Accel Y", "Accel Z"]
                .into_iter()
                .zip(&mut accelerometer)
            {
                changed |= momentary_motion_axis(ui, label, value, 0);
            }
            if changed {
                let motion = MotionSample {
                    gyroscope: gyro,
                    accelerometer,
                };
                let _ = controller.set_motion(motion);
            }
        });
    }
}

fn draw_dualshock4(ui: &mut egui::Ui, controller: &mut DualShock4Controller) {
    surface(ui, controller.surface());
    digital_controls(ui, |update| {
        let _ = controller.set_digital(update);
    });
    ui.group(|ui| {
        ui.label("Sticks and triggers");
        let (left_x, left_y) = controller.state().left_stick();
        let mut x = dualsense_axis_to_pad(left_x.raw());
        let mut y = dualsense_axis_to_pad(left_y.raw());
        if axis_pad(ui, "DualShock 4 left stick", &mut x, &mut y) {
            let _ = controller.set_left_stick(
                DualShock4Axis::new(dualsense_axis_from_pad(x)),
                DualShock4Axis::new(dualsense_axis_from_pad(y)),
            );
        }
        let (right_x, right_y) = controller.state().right_stick();
        let mut right_x = dualsense_axis_to_pad(right_x.raw());
        let mut right_y = dualsense_axis_to_pad(right_y.raw());
        if axis_pad(ui, "DualShock 4 right stick", &mut right_x, &mut right_y) {
            let _ = controller.set_right_stick(
                DualShock4Axis::new(dualsense_axis_from_pad(right_x)),
                DualShock4Axis::new(dualsense_axis_from_pad(right_y)),
            );
        }
        let (left, right) = controller.state().triggers();
        let mut left = left.raw();
        let mut right = right.raw();
        if momentary_trigger(ui, "L2", &mut left) | momentary_trigger(ui, "R2", &mut right) {
            let _ = controller
                .set_triggers(DualShock4Trigger::new(left), DualShock4Trigger::new(right));
        }
    });
    ui.group(|ui| {
        ui.label("Additional buttons");
        ui.horizontal_wrapped(|ui| {
            for (label, control) in [
                ("L1", DualShock4Control::L1),
                ("R1", DualShock4Control::R1),
                ("Share", DualShock4Control::Share),
                ("Options", DualShock4Control::Options),
                ("PlayStation", DualShock4Control::PlayStation),
                ("Touchpad click", DualShock4Control::TouchpadClick),
                ("Left stick press", DualShock4Control::LeftStickPress),
                ("Right stick press", DualShock4Control::RightStickPress),
            ] {
                hold(ui, label, |pressed| {
                    let _ = controller.set_native(control, pressed);
                });
            }
        });
    });
    ui.group(|ui| {
        ui.label("Touchpad");
        draw_ds4_touchpad(ui, controller);
        draw_ds4_touch_slot(
            ui,
            controller,
            DualShock4TouchSlot::Second,
            1,
            "Second contact",
        );
    });
    ui.group(|ui| {
        ui.label("UHID motion report");
        ui.small(
            "Motion controls are momentary and return to zero; no gravity orientation is implied.",
        );
        let motion = controller.state().motion();
        let mut gyro = motion.gyroscope;
        let mut accel = motion.accelerometer;
        let mut changed = false;
        for (label, value) in ["Gyro X", "Gyro Y", "Gyro Z"].into_iter().zip(&mut gyro) {
            changed |= momentary_motion_axis(ui, label, value, 0);
        }
        for (label, value) in ["Accel X", "Accel Y", "Accel Z"]
            .into_iter()
            .zip(&mut accel)
        {
            changed |= momentary_motion_axis(ui, label, value, 0);
        }
        if changed {
            let _ = controller.set_motion(DualShock4MotionSample {
                accelerometer: accel,
                gyroscope: gyro,
            });
        }
    });
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn draw_ds4_touchpad(ui: &mut egui::Ui, controller: &mut DualShock4Controller) {
    ui.small("Click and drag to emulate the first DualShock 4 touch contact.");
    let (rect, response) = ui.allocate_exact_size(Vec2::new(220.0, 125.0), Sense::click_and_drag());
    ui.painter().rect_stroke(
        rect,
        4.0,
        Stroke::new(1.0, Color32::GRAY),
        egui::StrokeKind::Inside,
    );
    if response.is_pointer_button_down_on() {
        if let Some(position) = response.interact_pointer_pos() {
            let x = ((position.x - rect.left()) / rect.width() * 1919.0).clamp(0.0, 1919.0) as u16;
            let y = ((position.y - rect.top()) / rect.height() * 941.0).clamp(0.0, 941.0) as u16;
            if let Ok(contact) = DualShock4TouchContact::new(0, x, y) {
                let _ = controller.set_touch(DualShock4TouchSlot::First, Some(contact));
            }
        }
    } else if response.drag_stopped() || response.clicked() {
        let _ = controller.set_touch(DualShock4TouchSlot::First, None);
    }
    for (slot, color) in [
        (DualShock4TouchSlot::First, Color32::LIGHT_BLUE),
        (DualShock4TouchSlot::Second, Color32::LIGHT_GREEN),
    ] {
        if let Some(contact) = controller.state().touch(slot) {
            let x = rect.left() + f32::from(contact.x()) / 1919.0 * rect.width();
            let y = rect.top() + f32::from(contact.y()) / 941.0 * rect.height();
            ui.painter().circle_filled(Pos2::new(x, y), 5.0, color);
        }
    }
}

fn draw_ds4_touch_slot(
    ui: &mut egui::Ui,
    controller: &mut DualShock4Controller,
    slot: DualShock4TouchSlot,
    id: u8,
    label: &str,
) {
    let contact = controller.state().touch(slot);
    let mut x = i32::from(contact.map_or(0, DualShock4TouchContact::x));
    let mut y = i32::from(contact.map_or(0, DualShock4TouchContact::y));
    ui.group(|ui| {
        ui.label(label);
        ui.add(egui::Slider::new(&mut x, 0..=1919).text("X"));
        ui.add(egui::Slider::new(&mut y, 0..=941).text("Y"));
        if ui.button("Set touch").clicked() {
            if let Ok(contact) = DualShock4TouchContact::new(
                id,
                u16::try_from(x).expect("slider bounds fit u16"),
                u16::try_from(y).expect("slider bounds fit u16"),
            ) {
                let _ = controller.set_touch(slot, Some(contact));
            }
        }
        if ui.button("Clear touch").clicked() {
            let _ = controller.set_touch(slot, None);
        }
    });
}

fn draw_switch_pro(ui: &mut egui::Ui, controller: &mut SwitchProController) {
    surface(ui, controller.surface());
    digital_controls(ui, |update| {
        let _ = controller.set_digital(update);
    });
    ui.group(|ui| {
        ui.label("Sticks");
        let (left_x, left_y) = controller.state().left_stick();
        let mut x = left_x.raw();
        let mut y = left_y.raw();
        if axis_pad(ui, "Switch Pro left stick", &mut x, &mut y) {
            let _ = controller.set_left_stick(SwitchProAxis::new(x), SwitchProAxis::new(y));
        }
        let (right_x, right_y) = controller.state().right_stick();
        let mut right_x = right_x.raw();
        let mut right_y = right_y.raw();
        if axis_pad(ui, "Switch Pro right stick", &mut right_x, &mut right_y) {
            let _ = controller
                .set_right_stick(SwitchProAxis::new(right_x), SwitchProAxis::new(right_y));
        }
    });
    ui.group(|ui| {
        ui.label("Additional buttons and triggers");
        ui.horizontal_wrapped(|ui| {
            for (label, control) in [
                ("L", SwitchProControl::L),
                ("R", SwitchProControl::R),
                ("ZL", SwitchProControl::Zl),
                ("ZR", SwitchProControl::Zr),
                ("Minus", SwitchProControl::Minus),
                ("Plus", SwitchProControl::Plus),
                ("Home", SwitchProControl::Home),
                ("Capture", SwitchProControl::Capture),
                ("Left stick press", SwitchProControl::LeftStickPress),
                ("Right stick press", SwitchProControl::RightStickPress),
            ] {
                hold(ui, label, |pressed| {
                    let _ = controller.set_native(control, pressed);
                });
            }
        });
    });
    ui.group(|ui| {
        ui.label("Switch Pro motion report");
        ui.small(if controller.state().stream_enabled() {
            format!(
                "Host selected report mode 0x30; streaming at 250 Hz (frame counter: {}).",
                controller.state().motion_report_counter()
            )
        } else {
            "Waiting for the host to select report mode 0x30.".to_owned()
        });
        let motion = controller.state().motion();
        let mut gyro = motion.gyroscope;
        let mut accel = motion.accelerometer;
        let mut changed = false;
        for (label, value) in ["Gyro X", "Gyro Y", "Gyro Z"].into_iter().zip(&mut gyro) {
            changed |= momentary_motion_axis(ui, label, value, 0);
        }
        for (label, value) in ["Accel X", "Accel Y", "Accel Z"]
            .into_iter()
            .zip(&mut accel)
        {
            changed |= momentary_motion_axis(ui, label, value, 0);
        }
        if changed {
            let _ = controller.set_motion(SwitchProMotionSample {
                accelerometer: accel,
                gyroscope: gyro,
            });
        }
    });
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn draw_touchpad(ui: &mut egui::Ui, controller: &mut DualSenseController) {
    ui.small("Click and drag to emulate the first physical touch contact.");
    let (rect, response) = ui.allocate_exact_size(Vec2::new(220.0, 125.0), Sense::click_and_drag());
    ui.painter().rect_stroke(
        rect,
        4.0,
        Stroke::new(1.0, Color32::GRAY),
        egui::StrokeKind::Inside,
    );
    if response.is_pointer_button_down_on() {
        if let Some(position) = response.interact_pointer_pos() {
            let x = ((position.x - rect.left()) / rect.width() * 1919.0).clamp(0.0, 1919.0) as u16;
            let y = ((position.y - rect.top()) / rect.height() * 941.0).clamp(0.0, 941.0) as u16;
            if let Ok(contact) = DualSenseTouchContact::new(0, x, y) {
                let _ = controller.set_touch(TouchSlot::First, Some(contact));
            }
        }
    } else if response.drag_stopped() || response.clicked() {
        let _ = controller.set_touch(TouchSlot::First, None);
    }
    for (slot, color) in [
        (TouchSlot::First, Color32::LIGHT_BLUE),
        (TouchSlot::Second, Color32::LIGHT_GREEN),
    ] {
        if let Some(contact) = controller.state().touch(slot) {
            let x = rect.left() + f32::from(contact.x()) / 1919.0 * rect.width();
            let y = rect.top() + f32::from(contact.y()) / 941.0 * rect.height();
            ui.painter().circle_filled(Pos2::new(x, y), 5.0, color);
        }
    }
}

fn draw_touch_slot(
    ui: &mut egui::Ui,
    controller: &mut DualSenseController,
    slot: TouchSlot,
    id: u8,
    label: &str,
) {
    let contact = controller.state().touch(slot);
    let mut x = i32::from(contact.map_or(0, DualSenseTouchContact::x));
    let mut y = i32::from(contact.map_or(0, DualSenseTouchContact::y));
    ui.group(|ui| {
        ui.label(label);
        ui.add(egui::Slider::new(&mut x, 0..=1919).text("X"));
        ui.add(egui::Slider::new(&mut y, 0..=941).text("Y"));
        if ui.button("Set touch").clicked() {
            if let Ok(contact) = DualSenseTouchContact::new(
                id,
                u16::try_from(x).expect("slider is bounded to u16"),
                u16::try_from(y).expect("slider is bounded to u16"),
            ) {
                let _ = controller.set_touch(slot, Some(contact));
            }
        }
        if ui.button("Clear touch").clicked() {
            let _ = controller.set_touch(slot, None);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motion_worker_uses_the_advertised_250_hz_interval() {
        assert_eq!(motion_worker_interval(), Duration::from_millis(4));
    }

    #[test]
    fn live_controllers_poll_reverse_output_at_the_usb_cadence() {
        assert_eq!(repaint_interval(0), Duration::from_millis(50));
        assert_eq!(repaint_interval(1), Duration::from_millis(4));
        assert_eq!(repaint_interval(8), Duration::from_millis(4));
    }

    #[test]
    fn dualsense_motion_refresh_is_available_for_uhid_and_dummy_hcd() {
        assert!(dualsense_motion_target(RealizationTarget::Uhid));
        assert!(dualsense_motion_target(RealizationTarget::DummyHcd));
        assert!(!dualsense_motion_target(RealizationTarget::Evdev));
        assert_eq!(
            dualsense_motion_target_label(RealizationTarget::Uhid),
            "UHID motion report"
        );
        assert_eq!(
            dualsense_motion_target_label(RealizationTarget::DummyHcd),
            "DummyHcd USB motion report"
        );
    }

    #[test]
    fn accel_z_returns_to_gui_neutral_without_publishing_gravity() {
        let mut value = -12_000;
        assert!(reset_momentary_motion_axis(&mut value, 0));
        assert_eq!(value, 0);
        assert!(!reset_momentary_motion_axis(&mut value, 0));
    }

    #[test]
    fn provider_failure_status_names_the_closed_controller() {
        assert_eq!(
            status_after_runtime_failure("DualSense 0", "provider closed".into()),
            ControllerLifecycleStatus::ClosedAfterFailure {
                name: "DualSense 0".into(),
                error: "provider closed".into(),
            }
        );
    }

    #[test]
    fn removing_any_tab_selects_the_nearest_remaining_controller() {
        assert_eq!(selection_after_removal(0, 0), None);
        assert_eq!(selection_after_removal(2, 0), Some(0));
        assert_eq!(selection_after_removal(2, 1), Some(1));
        assert_eq!(selection_after_removal(2, 2), Some(1));
    }

    #[test]
    fn tab_list_keeps_controllers_beyond_the_third_position() {
        assert_eq!(
            controller_tab_indices(4).collect::<Vec<_>>(),
            vec![0, 1, 2, 3]
        );
        assert_eq!(controller_tab_indices(12).count(), 12);
    }

    #[test]
    fn rumble_only_dualsense_output_preserves_prior_led_indicators() {
        let mut indicators = ReverseIndicators {
            led: Some([0x11, 0x22, 0x33]),
            mute_led: Some(true),
            ..ReverseIndicators::default()
        };
        indicators.apply_dualsense_usb_output(Some(0x40), Some(0x20), None, None);
        assert_eq!(indicators.led, Some([0x11, 0x22, 0x33]));
        assert_eq!(indicators.mute_led, Some(true));
        assert!(indicators.rumble_active);
    }
}
