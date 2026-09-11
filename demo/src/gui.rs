mod editor;
use editor::{
    Command, ControllerView, DualSenseEditor, DualShock4Editor, SwitchProEditor, Xbox360Editor,
};
use eframe::egui::{self, Button, Color32, Pos2, Sense, Stroke, Vec2};
use std::{
    sync::{Arc, Mutex, mpsc},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use virtualgamepad::ControllerSurfaceInfo;
use virtualgamepad::{
    BatteryLevel, BatteryState, DigitalControlUpdate, DpadDirection, DualSenseAxis,
    DualSenseControl, DualSenseController, DualSenseHidOutput, DualSenseOutputEvent,
    DualSenseTouchContact, DualSenseTrigger, DualShock4Axis, DualShock4Control,
    DualShock4Controller, DualShock4HidOutput, DualShock4MotionSample, DualShock4TouchContact,
    DualShock4TouchSlot, DualShock4Trigger, FaceButton, MotionSample, RealizationId, SwitchProAxis,
    SwitchProControl, SwitchProController, SwitchProMotionSample, TouchSlot, Xbox360Axis,
    Xbox360Control, Xbox360Controller, Xbox360OutputEvent, Xbox360Trigger, create_dualsense,
    create_dualshock4, create_switch_pro, create_xbox360,
};

// Lab correlation is application bookkeeping, never controller/session identity.
#[derive(Clone, Copy)]
struct LabOptions {
    target: RealizationId,
    session: u64,
}

const OUTPUT_LOG_LIMIT: usize = 200;
const DUALSENSE_MOTION_INTERVAL: Duration = Duration::from_millis(4);
const IDLE_REPAINT_INTERVAL: Duration = Duration::from_millis(50);

fn dualsense_motion_target(target: RealizationId) -> bool {
    matches!(
        target,
        RealizationId::LINUX_UHID_USB | RealizationId::LINUX_DUMMY_HCD_USB_HID
    )
}

fn dualsense_motion_target_label(target: RealizationId) -> &'static str {
    if target == RealizationId::LINUX_UHID_USB {
        "UHID motion report"
    } else {
        "DummyHcd USB motion report"
    }
}

fn motion_refresh_target(target: RealizationId) -> bool {
    matches!(
        target,
        RealizationId::LINUX_UHID_USB | RealizationId::LINUX_DUMMY_HCD_USB_HID
    )
}

fn repaint_interval(controller_count: usize) -> Duration {
    if controller_count == 0 {
        IDLE_REPAINT_INTERVAL
    } else {
        DUALSENSE_MOTION_INTERVAL
    }
}

fn service_repaint_interval(controller_count: usize, next_service: Option<Duration>) -> Duration {
    let fallback = repaint_interval(controller_count);
    next_service.map_or(fallback, |deadline| deadline.min(fallback))
}

const fn motion_worker_interval() -> Duration {
    DUALSENSE_MOTION_INTERVAL
}

const fn following_session(current: u64, advance: bool) -> u64 {
    if advance {
        current.wrapping_add(1)
    } else {
        current
    }
}

#[derive(Default)]
struct ConsumerNotes {
    build: String,
    backend: String,
    mapping: String,
}

fn lab_record(
    name: &str,
    options: LabOptions,
    metrics: &ServiceMetrics,
    notes: &str,
    consumer: &ConsumerNotes,
    details: &str,
) -> String {
    format!(
        "Virtualgamepad manual lab record v3\nController: {name}\nRealization: {}\nLab correlation: {}\nService cycles: {}\nMaximum observed service gap (us): {}\nOmitted worker logs: {}\nConsumer build: {}\nConsumer backend: {}\nConsumer mapping: {}\nSession diagnostics: {details}\nPhysical/consumer acceptance: not established by this record\nNotes:\n{notes}",
        target_label(options.target),
        options.session,
        metrics.cycles,
        metrics.max_gap.as_micros(),
        metrics.omitted_logs,
        consumer.build,
        consumer.backend,
        consumer.mapping
    )
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

fn creation_error_message(
    error: &virtualgamepad::ControllerError,
    uhid_registered: Option<bool>,
) -> String {
    use virtualgamepad::ControllerError;
    let message = error.to_string();
    let (ControllerError::AccessDenied { target, path }
    | ControllerError::MissingDeviceNode { target, path }) = error
    else {
        return message;
    };
    if ![
        RealizationId::LINUX_UHID_USB,
        RealizationId::LINUX_UHID_BLUETOOTH,
    ]
    .contains(target)
        || path != "/dev/uhid"
    {
        return message;
    }
    let guidance = if uhid_registered == Some(false) {
        "UHID kernel registration is missing. Have an administrator load the uhid module, then retry. If this recurs after reboot, configure UHID boot loading in host setup."
    } else {
        "Check UHID device access for this login. Temporary helper access lasts only for this boot; persistent access requires administrator-configured host policy."
    };
    format!("{message}. {guidance}")
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

#[derive(Default)]
struct EditProgress {
    submitted: u64,
    applied: u64,
}
impl EditProgress {
    fn ready(&self) -> bool {
        self.applied == self.submitted
    }
    fn observe(&mut self, applied: u64) -> bool {
        if applied != self.submitted {
            return false;
        }
        self.applied = applied;
        true
    }
    fn submit<C>(
        &mut self,
        sender: &mpsc::SyncSender<(u64, Vec<Command<C>>)>,
        edits: Vec<Command<C>>,
    ) -> Result<(), String> {
        if edits.is_empty() {
            return Ok(());
        }
        if !self.ready() {
            return Err("previous input batch is still pending".into());
        }
        let next = self
            .submitted
            .checked_add(1)
            .ok_or("edit sequence exhausted")?;
        sender.try_send((next, edits)).map_err(|_| {
            "edit queue unavailable; controller closed to avoid lost releases".to_owned()
        })?;
        self.submitted = next;
        Ok(())
    }
}

struct NamedController {
    kind: Kind,
    options: LabOptions,
    name: String,
    view: ControllerView,
    edits: EditProgress,
    indicators: ReverseIndicators,
    service_worker: Option<ServiceWorker<Controller>>,
    second_touch: LatchedTouch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LatchedTouch {
    active: bool,
    x: u16,
    y: u16,
}

impl Default for LatchedTouch {
    fn default() -> Self {
        Self {
            active: false,
            x: 960,
            y: 470,
        }
    }
}

impl LatchedTouch {
    fn contact(self, id: u8) -> Option<DualSenseTouchContact> {
        self.active
            .then(|| DualSenseTouchContact::new(id, self.x, self.y))
            .transpose()
            .expect("latched touch coordinates are bounded by the GUI sliders")
    }
}

#[derive(Clone, Default)]
struct ServiceMetrics {
    cycles: u64,
    max_gap: Duration,
    omitted_logs: u64,
    last_service: Option<Instant>,
}
impl ServiceMetrics {
    fn record(&mut self, now: Instant) {
        if let Some(previous) = self.last_service {
            self.max_gap = self.max_gap.max(now.saturating_duration_since(previous));
        }
        self.last_service = Some(now);
        self.cycles = self.cycles.saturating_add(1);
    }
    fn omit(&mut self, count: usize) {
        self.omitted_logs = self
            .omitted_logs
            .saturating_add(u64::try_from(count).unwrap_or(u64::MAX));
    }
}

#[derive(Default)]
struct WorkerDisplay {
    snapshot: Option<ControllerView>,
    applied: u64,
    logs: Vec<String>,
    metrics: ServiceMetrics,
    indicators: ReverseIndicators,
}

struct ServiceWorker<C> {
    edits: mpsc::SyncSender<(u64, Vec<Command<C>>)>,
    stop: mpsc::Sender<()>,
    failure: mpsc::Receiver<String>,
    display: Arc<Mutex<WorkerDisplay>>,
    handle: JoinHandle<C>,
}

fn worker_failure(receiver: &mpsc::Receiver<String>) -> Option<String> {
    match receiver.try_recv() {
        Ok(error) => Some(error),
        Err(mpsc::TryRecvError::Empty) => None,
        Err(mpsc::TryRecvError::Disconnected) => {
            Some("controller worker exited unexpectedly".into())
        }
    }
}

impl<C> ServiceWorker<C> {
    fn stop(self) -> Option<C> {
        let _ = self.stop.send(());
        self.handle.join().ok()
    }
}

trait ServicedController: Send + Sized {
    fn snapshot(&mut self) -> Option<ControllerView> {
        None
    }
    fn commit_edits(&mut self) -> Result<(), String> {
        Ok(())
    }
    fn apply(&mut self, edits: Vec<Command<Self>>) -> Result<(), String> {
        if edits.len() > editor::EDIT_LIMIT {
            return Err("native edit batch exceeded its bound".into());
        }
        for edit in edits {
            edit(self)?;
        }
        self.commit_edits()
    }
    fn neutralize(&mut self) -> Result<(), String>;
    fn refresh(&mut self) -> Result<(), String>;
    fn service(
        &mut self,
        log: &mut Vec<String>,
        indicators: &mut ReverseIndicators,
    ) -> Result<(), String>;
    fn deadline(&self) -> Option<Duration>;
    fn close(&mut self);
}
fn release_inputs<C: ServicedController + 'static>() -> Command<C> {
    Box::new(ServicedController::neutralize)
}

impl ServicedController for Controller {
    fn neutralize(&mut self) -> Result<(), String> {
        match self {
            Self::Xbox(c) => c.neutralize(),
            Self::DualSense(c) => c.neutralize(),
            Self::DualShock4(c) => c.neutralize(),
            Self::SwitchPro(c) => c.neutralize(),
        }
        .map_err(|error| error.to_string())
    }
    fn snapshot(&mut self) -> Option<ControllerView> {
        Some(Self::snapshot(self))
    }
    fn commit_edits(&mut self) -> Result<(), String> {
        if self.is_dirty() {
            self.commit()?;
        }
        Ok(())
    }
    fn refresh(&mut self) -> Result<(), String> {
        self.refresh_motion()
    }
    fn service(
        &mut self,
        log: &mut Vec<String>,
        indicators: &mut ReverseIndicators,
    ) -> Result<(), String> {
        self.poll_output(log, indicators)
    }
    fn deadline(&self) -> Option<Duration> {
        self.next_service_in()
    }
    fn close(&mut self) {
        Self::close(self);
    }
}

fn service_cycle<C: ServicedController>(
    controller: &mut C,
    now: Duration,
    next_motion: &mut Duration,
    logs: &mut Vec<String>,
    indicators: &mut ReverseIndicators,
) -> Result<Duration, String> {
    if now >= *next_motion {
        controller.refresh()?;
        *next_motion = now.saturating_add(motion_worker_interval());
    }
    controller.service(logs, indicators)?;
    Ok(controller
        .deadline()
        .unwrap_or(motion_worker_interval())
        .min(next_motion.saturating_sub(now)))
}

fn publish_display(display: &mut WorkerDisplay, logs: Vec<String>, indicators: &ReverseIndicators) {
    display.logs.extend(logs);
    let excess = display.logs.len().saturating_sub(OUTPUT_LOG_LIMIT);
    display.logs.drain(..excess);
    display.indicators = indicators.clone();
}

fn spawn_service_worker<C: ServicedController + 'static>(mut controller: C) -> ServiceWorker<C> {
    let (stop_sender, stop_receiver) = mpsc::channel();
    let (edit_sender, edit_receiver) = mpsc::sync_channel::<(u64, Vec<Command<C>>)>(1);
    let (failure_sender, failure_receiver) = mpsc::sync_channel(1);
    let display = Arc::new(Mutex::new(WorkerDisplay::default()));
    let worker_display = Arc::clone(&display);

    let handle = thread::spawn(move || {
        let start = Instant::now();
        let mut next_motion = Duration::ZERO;
        let mut delay = Duration::ZERO;
        let mut indicators = ReverseIndicators::default();
        let mut metrics = ServiceMetrics::default();
        let mut applied = 0;
        loop {
            match stop_receiver.recv_timeout(delay) {
                Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
            let mut logs = Vec::new();
            let result = (|| {
                // At most one bounded edit batch per service cycle. Stop has a
                // separate channel and is checked before queued input.
                if let Ok((sequence, edits)) = edit_receiver.try_recv() {
                    controller.apply(edits)?;
                    applied = sequence;
                }
                service_cycle(
                    &mut controller,
                    start.elapsed(),
                    &mut next_motion,
                    &mut logs,
                    &mut indicators,
                )
            })();
            // Optional UI output never owns or delays protocol replies.
            metrics.record(Instant::now());
            if let Ok(mut display) = worker_display.try_lock() {
                metrics.omit(
                    display
                        .logs
                        .len()
                        .saturating_add(logs.len())
                        .saturating_sub(OUTPUT_LOG_LIMIT),
                );
                publish_display(&mut display, logs, &indicators);
                display.metrics = metrics.clone();
                display.snapshot = controller.snapshot();
                display.applied = applied;
            } else {
                metrics.omit(logs.len());
            }
            match result {
                Ok(next) => delay = next.min(next_motion.saturating_sub(start.elapsed())),
                Err(error) => {
                    let _ = failure_sender.try_send(error);
                    break;
                }
            }
        }
        // Stop or failure closes immediately, even when the UI never repaints.
        controller.close();
        controller
    });
    ServiceWorker {
        edits: edit_sender,
        stop: stop_sender,
        failure: failure_receiver,
        display,
        handle,
    }
}

#[derive(Clone, Default)]
struct ReverseIndicators {
    led: Option<[u8; 3]>,
    mute_led: Option<bool>,
    rumble_until: Option<Instant>,
    rumble_active: bool,
    hid_motors: [u8; 2],
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
    fn apply_force_feedback(&mut self, event: virtualgamepad::ForceFeedbackEvent) {
        if let virtualgamepad::ForceFeedbackEvent::Playback {
            effect,
            repetitions,
        } = event
        {
            // Activity pulse, not a simulation of replay timing or physical motors.
            if repetitions != 0 && (effect.strong != 0 || effect.weak != 0) {
                self.rumble_pulse();
            } else {
                self.set_rumble(false);
                self.rumble_until = None;
            }
        }
    }
    fn apply_hid_output(
        &mut self,
        right_motor: Option<u8>,
        left_motor: Option<u8>,
        lightbar_rgb: Option<[u8; 3]>,
        mute_button_led: Option<bool>,
    ) {
        if let Some(value) = right_motor {
            self.hid_motors[0] = value;
        }
        if let Some(value) = left_motor {
            self.hid_motors[1] = value;
        }
        if right_motor.is_some() || left_motor.is_some() {
            self.set_rumble(self.hid_motors.iter().any(|value| *value != 0));
        }
        if let Some(lightbar_rgb) = lightbar_rgb {
            self.led = Some(lightbar_rgb);
        }
        if let Some(mute_button_led) = mute_button_led {
            self.mute_led = Some(mute_button_led);
        }
    }
}
impl Controller {
    fn next_service_in(&self) -> Option<Duration> {
        match self {
            Self::Xbox(controller) => controller.next_service_in(),
            Self::DualSense(controller) => controller.next_service_in(),
            Self::DualShock4(controller) => controller.next_service_in(),
            Self::SwitchPro(controller) => controller.next_service_in(),
        }
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
                if motion_refresh_target(controller.surface().common().target) =>
            {
                controller
                    .set_motion(controller.state().motion())
                    .map_err(|error| error.to_string())?;
                controller.commit().map_err(|error| error.to_string())
            }
            Self::SwitchPro(controller)
                if motion_refresh_target(controller.surface().common().target) =>
            {
                controller.commit().map_err(|error| error.to_string())
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
    fn poll_output(
        &mut self,
        log: &mut Vec<String>,
        indicators: &mut ReverseIndicators,
    ) -> Result<(), String> {
        let result: Result<(), String> = match self {
            Self::Xbox(controller) => controller
                .service(&mut |event| {
                    if let Xbox360OutputEvent::ForceFeedback(event) = event {
                        indicators.apply_force_feedback(event);
                    }
                    log.push(format!("Xbox 360: {event:?}"));
                })
                .map_err(|error| error.to_string()),
            Self::DualSense(controller) => controller
                .service(&mut |event| {
                    match &event {
                        DualSenseOutputEvent::ForceFeedback(event) => {
                            indicators.apply_force_feedback(*event);
                        }
                        DualSenseOutputEvent::HidOutput(DualSenseHidOutput::UsbOutput {
                            right_motor,
                            left_motor,
                            lightbar_rgb,
                            mute_button_led,
                            ..
                        }) => indicators.apply_hid_output(
                            *right_motor,
                            *left_motor,
                            *lightbar_rgb,
                            *mute_button_led,
                        ),
                        _ => {}
                    }
                    log.push(format!("DualSense: {event:?}"));
                })
                .map_err(|error| error.to_string()),
            Self::DualShock4(controller) => controller
                .service(&mut |event| {
                    if let virtualgamepad::DualShock4OutputEvent::HidOutput(
                        DualShock4HidOutput::UsbOutput {
                            right_motor,
                            left_motor,
                            lightbar_rgb,
                            ..
                        },
                    ) = &event
                    {
                        indicators.apply_hid_output(
                            Some(*right_motor),
                            Some(*left_motor),
                            *lightbar_rgb,
                            None,
                        );
                    }
                    if let virtualgamepad::DualShock4OutputEvent::ForceFeedback(event) = event {
                        indicators.apply_force_feedback(event);
                    }
                    log.push(format!("DualShock 4: {event:?}"));
                })
                .map_err(|error| error.to_string()),
            Self::SwitchPro(controller) => controller
                .service(&mut |event| {
                    if let virtualgamepad::SwitchProOutputEvent::ForceFeedback(event) = event {
                        indicators.apply_force_feedback(event);
                    }
                    log.push(format!("Switch Pro: {event:?}"));
                })
                .map_err(|error| error.to_string()),
        };
        result
    }
}
impl ControllerView {
    fn draw(&mut self, ui: &mut egui::Ui, second_touch: &mut LatchedTouch) {
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
            Self::DualSense(controller) => draw_dualsense(ui, controller, second_touch),
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
    }
    fn set_battery_level(&mut self, level: BatteryLevel) -> Result<(), String> {
        match self {
            Self::Xbox(controller) => controller.set_battery_level(level),
            Self::DualSense(controller) => controller.set_battery_level(level),
            Self::DualShock4(_) | Self::SwitchPro(_) => Ok(()),
        }
    }
}
pub struct App {
    kind: Kind,
    target: RealizationId,
    name_draft: String,
    next_session: u64,
    advance_session: bool,
    lab_notes: String,
    consumer_notes: ConsumerNotes,
    last_cleanup: Option<String>,
    controllers: Vec<NamedController>,
    selected_controller: Option<usize>,
    output_log: Vec<String>,
    lifecycle_status: Option<ControllerLifecycleStatus>,
}
impl Default for App {
    fn default() -> Self {
        Self {
            kind: Kind::Xbox360,
            target: RealizationId::LINUX_UINPUT,
            name_draft: String::new(),
            next_session: 1,
            advance_session: true,
            lab_notes: String::new(),
            consumer_notes: ConsumerNotes::default(),
            last_cleanup: None,
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
        let options = LabOptions {
            target: self.target,
            session: self.next_session,
        };
        let result = match self.kind {
            Kind::Xbox360 => create_xbox360(virtualgamepad::CreationOptions::new(options.target))
                .map(Controller::Xbox),
            Kind::DualSense => {
                create_dualsense(virtualgamepad::CreationOptions::new(options.target))
                    .map(Controller::DualSense)
            }
            Kind::DualShock4 => {
                create_dualshock4(virtualgamepad::CreationOptions::new(options.target))
                    .map(Controller::DualShock4)
            }
            Kind::SwitchPro => {
                create_switch_pro(virtualgamepad::CreationOptions::new(options.target))
                    .map(Controller::SwitchPro)
            }
        };
        match result {
            Ok(mut controller) => {
                let name = if self.name_draft.trim().is_empty() {
                    self.next_default_name()
                } else {
                    self.name_draft.trim().to_owned()
                };
                let view = controller.snapshot();
                let service_worker = Some(spawn_service_worker(controller));
                self.controllers.push(NamedController {
                    kind: self.kind,
                    options,
                    name: name.clone(),
                    view,
                    edits: EditProgress::default(),
                    indicators: ReverseIndicators::default(),
                    service_worker,
                    second_touch: LatchedTouch::default(),
                });
                self.selected_controller = Some(self.controllers.len() - 1);
                self.name_draft.clear();
                self.next_session = following_session(self.next_session, self.advance_session);
                self.lifecycle_status = Some(ControllerLifecycleStatus::Created { name });
            }
            Err(error) => {
                let error = creation_error_message(
                    &error,
                    std::path::Path::new("/sys/class/misc/uhid/dev")
                        .try_exists()
                        .ok(),
                );
                self.lifecycle_status = Some(ControllerLifecycleStatus::CreationFailed { error });
            }
        }
    }

    fn remove_controller(&mut self, index: usize) {
        if index >= self.controllers.len() {
            return;
        }
        let mut removed = self.controllers.remove(index);
        if let Some(worker) = removed.service_worker.take() {
            self.last_cleanup = Some(worker.stop().map_or_else(
                || "Worker exited without a returned controller; host cleanup requires verification".into(),
                |mut controller| controller.snapshot().lab_details(),
            ));
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
            if let Some(worker) = named.service_worker.take() {
                worker.stop();
            }
        }
    }
}
impl eframe::App for App {
    #[allow(clippy::too_many_lines)] // Coordinates the independent demo panels.
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        let mut remove = None;
        let mut stop_all = false;
        let mut failed_controller = None;
        for (index, named) in self.controllers.iter_mut().enumerate() {
            if let Some(worker) = &named.service_worker {
                if let Some(error) = worker_failure(&worker.failure) {
                    failed_controller = Some((index, error));
                    break;
                }
            }
            if let Some(worker) = &named.service_worker {
                if let Ok(mut display) = worker.display.try_lock() {
                    self.output_log.append(&mut display.logs);
                    named.indicators = display.indicators.clone();
                    if display.snapshot.is_some() && named.edits.observe(display.applied) {
                        named.view = display.snapshot.take().expect("checked snapshot");
                    }
                }
            }
        }
        if self.output_log.len() > OUTPUT_LOG_LIMIT {
            let excess = self.output_log.len() - OUTPUT_LOG_LIMIT;
            self.output_log.drain(..excess);
        }
        ctx.request_repaint_after(service_repaint_interval(self.controllers.len(), None));
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
                        RealizationId::LINUX_UINPUT,
                        target_label(RealizationId::LINUX_UINPUT),
                    );
                    ui.selectable_value(
                        &mut self.target,
                        RealizationId::LINUX_UHID_USB,
                        target_label(RealizationId::LINUX_UHID_USB),
                    );
                    ui.add_enabled(false, egui::Button::new("USB gadget (experimental)"))
                        .on_disabled_hover_text("Gate G is unresolved: required USB requests still need the research protocol API.");
                });
            let default_name = self.next_default_name();
            ui.add(
                egui::TextEdit::singleline(&mut self.name_draft)
                    .hint_text(default_name)
                    .desired_width(f32::INFINITY),
            )
            .on_hover_text("Optional name. Leave empty for the automatic controller name.");
            ui.small("UHID requires administrator-prepared device access.");
            if ui.button("Create").clicked() {
                self.create();
            }
            if ui.button("Stop all controllers").clicked() { stop_all = true; }
            ui.collapsing("Lab notes and gate prerequisites", |ui| {
            ui.horizontal(|ui| {
                ui.label("Lab correlation ID");
                ui.add(egui::DragValue::new(&mut self.next_session));
            });
            ui.checkbox(&mut self.advance_session, "Advance ID after creation");
            ui.small("This ID labels lab records only. Controller identity and session tokens are library-owned.");
                ui.label("Consumer build/version");
                ui.text_edit_singleline(&mut self.consumer_notes.build);
                ui.label("Input backend (for example SDL HIDAPI or Linux event)");
                ui.text_edit_singleline(&mut self.consumer_notes.backend);
                ui.label("Observed mapping/profile");
                ui.text_edit_multiline(&mut self.consumer_notes.mapping);
                ui.label("Observations");
                ui.text_edit_multiline(&mut self.lab_notes);
                if let Some(cleanup) = &self.last_cleanup {
                    ui.label(format!("Last removed session: {cleanup}"));
                    if ui.button("Copy cleanup diagnostics").clicked() { ui.ctx().copy_text(cleanup.clone()); }
                }
                ui.small("Record reference model, firmware, USB/BT mode, consumer/version and observed result.");
                ui.small("References: DualSense, Xbox Series, Steam Controller. Other families: best-effort.");
                ui.small("DS4 split touch is test-only; isolated consumers are required before live acceptance.");
                ui.small("Gadget: run scripts/host-preflight.py first. Socket access alone does not pass Gate G.");
            });
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
                            ui.label(format!("{} · application ID {}", target_label(named.options.target), named.options.session));
                            if let Some(worker) = &named.service_worker {
                                if let Ok(display) = worker.display.try_lock() {
                                    ui.label(format!("Service cycles: {} · max observed gap: {:.2} ms · omitted worker logs: {}",
                                        display.metrics.cycles, display.metrics.max_gap.as_secs_f64() * 1000.0, display.metrics.omitted_logs));
                                    if ui.button("Copy lab record").clicked() {
                                        ui.ctx().copy_text(lab_record(&named.name, named.options, &display.metrics, &self.lab_notes, &self.consumer_notes, &named.view.lab_details()));
                                    }
                                }
                            }
                            if named.edits.ready() {
                                if ui.button("Release all inputs").clicked() {
                                    named.second_touch.active = false;
                                    if let Err(error) = named.view.release_inputs() { failed_controller = Some((index, error)); }
                                } else {
                                    named.view.draw(ui, &mut named.second_touch);
                                }
                                let result = named.view.take_edits().and_then(|edits| {
                                    let worker = named.service_worker.as_ref().ok_or("worker unavailable")?;
                                    named.edits.submit(&worker.edits, edits)
                                });
                                if let Err(error) = result { failed_controller = Some((index, error)); }
                            } else {
                                ui.small("Waiting for the previous input batch; servicing continues independently.");
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
                    ui.small("Background service uses a 4 ms fallback and earlier deadlines. This bounded log is observational, not an acceptance verdict.");
                    if self.output_log.is_empty() {
                        ui.small("No reverse output received.");
                    }
                    for entry in self.output_log.iter().rev().take(20) {
                        ui.monospace(entry);
                    }
                });
        });
        if stop_all {
            while !self.controllers.is_empty() {
                self.remove_controller(self.controllers.len() - 1);
            }
        } else if let Some((index, error)) = failed_controller {
            self.close_failed_controller(index, error);
        } else if let Some(index) = remove {
            self.remove_controller(index);
        }
    }
}

fn target_label(target: RealizationId) -> &'static str {
    match target {
        RealizationId::LINUX_UINPUT => "Evdev / uinput",
        RealizationId::LINUX_UHID_USB => "HID / UHID",
        RealizationId::LINUX_DUMMY_HCD_USB_HID => "USB / dummy_hcd",
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

const fn face_labels(kind: Kind) -> [&'static str; 4] {
    match kind {
        Kind::Xbox360 => ["A (South)", "B (East)", "X (West)", "Y (North)"],
        Kind::DualSense | Kind::DualShock4 => [
            "Cross (South)",
            "Circle (East)",
            "Square (West)",
            "Triangle (North)",
        ],
        Kind::SwitchPro => ["B (South)", "A (East)", "Y (West)", "X (North)"],
    }
}

fn digital_controls(ui: &mut egui::Ui, kind: Kind, mut set: impl FnMut(DigitalControlUpdate)) {
    ui.group(|ui| {
        ui.label("Face buttons");
        ui.horizontal_wrapped(|ui| {
            for (label, button) in [
                (face_labels(kind)[0], FaceButton::South),
                (face_labels(kind)[1], FaceButton::East),
                (face_labels(kind)[2], FaceButton::West),
                (face_labels(kind)[3], FaceButton::North),
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
    let previous = ui
        .data(|data| data.get_temp::<bool>(response.id))
        .unwrap_or(false);
    if let Some(next) = next_hold_state(
        previous,
        response.is_pointer_button_down_on(),
        response.clicked(),
    ) {
        ui.data_mut(|data| data.insert_temp(response.id, next));
        set(next);
    }
}

fn next_hold_state(previous: bool, pointer_down: bool, clicked: bool) -> Option<bool> {
    // A quick click may begin and end between rendered frames. Keep that click
    // pressed for one complete frame so HID consumers observe a rising edge;
    // the following frame emits the corresponding release.
    let next = pointer_down || clicked;
    (next != previous).then_some(next)
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
                let next_x =
                    pad_axis_from_fraction((position.x - rect.center().x) / (rect.width() / 2.0));
                let next_y =
                    pad_axis_from_fraction((position.y - rect.center().y) / (rect.height() / 2.0));
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

fn latched_motion_axis(ui: &mut egui::Ui, label: &str, value: &mut i16) -> bool {
    ui.add(egui::Slider::new(value, i16::MIN..=i16::MAX).text(label))
        .changed()
}

fn dualsense_axis_to_pad(value: u8) -> i16 {
    let offset = i32::from(value) - 128;
    let mapped = if offset <= 0 {
        offset * 256
    } else {
        (offset * 32767 + 63) / 127
    };
    i16::try_from(mapped).expect("unsigned axis maps into signed pad")
}

fn dualsense_axis_from_pad(value: i16) -> u8 {
    let value = i32::from(value);
    let mapped = if value <= 0 {
        (value + 32768 + 128) / 256
    } else {
        128 + (value * 127 + 16383) / 32767
    };
    u8::try_from(mapped).expect("signed pad maps into unsigned axis")
}

#[allow(clippy::cast_possible_truncation)] // Rounded bounded normalized input fits i16.
fn pad_axis_from_fraction(value: f32) -> i16 {
    let value = value.clamp(-1.0, 1.0);
    (value * if value < 0.0 { 32768.0 } else { 32767.0 }).round() as i16
}

fn draw_xbox(ui: &mut egui::Ui, controller: &mut Xbox360Editor) {
    surface(ui, controller.surface());
    digital_controls(ui, Kind::Xbox360, |update| {
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
fn draw_dualsense(
    ui: &mut egui::Ui,
    controller: &mut DualSenseEditor,
    second_touch: &mut LatchedTouch,
) {
    surface(ui, controller.surface());
    digital_controls(ui, Kind::DualSense, |update| {
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
        draw_latched_touch_slot(ui, controller, TouchSlot::Second, 1, second_touch);
    });
    let target = controller.surface().common().target;
    if dualsense_motion_target(target) {
        ui.group(|ui| {
            ui.label(dualsense_motion_target_label(target));
            let diagnostics = controller.diagnostics();
            ui.small(format!(
                "HID reports sent: {}; host requests handled: {}",
                diagnostics.frames_sent(),
                diagnostics.reverse_events_drained()
            ));
            let motion = controller.state().motion();
            let mut gyro = motion.gyroscope;
            let mut accelerometer = motion.accelerometer;
            let mut changed = false;
            for (label, value) in ["Gyro X", "Gyro Y", "Gyro Z"].into_iter().zip(&mut gyro) {
                changed |= latched_motion_axis(ui, label, value);
            }
            for (label, value) in ["Accel X", "Accel Y", "Accel Z"]
                .into_iter()
                .zip(&mut accelerometer)
            {
                changed |= latched_motion_axis(ui, label, value);
            }
            if ui.button("Reset gyro to neutral").clicked() {
                gyro = [0; 3];
                changed = true;
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

fn draw_dualshock4(ui: &mut egui::Ui, controller: &mut DualShock4Editor) {
    surface(ui, controller.surface());
    digital_controls(ui, Kind::DualShock4, |update| {
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
            "Motion controls retain their value; zero remains neutral with no implied gravity.",
        );
        let motion = controller.state().motion();
        let mut gyro = motion.gyroscope;
        let mut accel = motion.accelerometer;
        let mut changed = false;
        for (label, value) in ["Gyro X", "Gyro Y", "Gyro Z"].into_iter().zip(&mut gyro) {
            changed |= latched_motion_axis(ui, label, value);
        }
        for (label, value) in ["Accel X", "Accel Y", "Accel Z"]
            .into_iter()
            .zip(&mut accel)
        {
            changed |= latched_motion_axis(ui, label, value);
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
fn draw_ds4_touchpad(ui: &mut egui::Ui, controller: &mut DualShock4Editor) {
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
    controller: &mut DualShock4Editor,
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

fn draw_switch_pro(ui: &mut egui::Ui, controller: &mut SwitchProEditor) {
    surface(ui, controller.surface());
    digital_controls(ui, Kind::SwitchPro, |update| {
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
        ui.small(if controller.stream_enabled() {
            format!(
                "Host selected report mode 0x30; streaming at 250 Hz (frame counter: {}).",
                controller.motion_report_counter()
            )
        } else {
            "Waiting for the host to select report mode 0x30.".to_owned()
        });
        let motion = controller.state().motion();
        let mut gyro = motion.gyroscope;
        let mut accel = motion.accelerometer;
        let mut changed = false;
        for (label, value) in ["Gyro X", "Gyro Y", "Gyro Z"].into_iter().zip(&mut gyro) {
            changed |= latched_motion_axis(ui, label, value);
        }
        for (label, value) in ["Accel X", "Accel Y", "Accel Z"]
            .into_iter()
            .zip(&mut accel)
        {
            changed |= latched_motion_axis(ui, label, value);
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
fn draw_touchpad(ui: &mut egui::Ui, controller: &mut DualSenseEditor) {
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

fn draw_latched_touch_slot(
    ui: &mut egui::Ui,
    controller: &mut DualSenseEditor,
    slot: TouchSlot,
    id: u8,
    touch: &mut LatchedTouch,
) {
    ui.group(|ui| {
        ui.label("Second contact");
        let mut changed = ui.checkbox(&mut touch.active, "Active").changed();
        changed |= ui
            .add(egui::Slider::new(&mut touch.x, 0..=1919).text("X"))
            .changed();
        changed |= ui
            .add(egui::Slider::new(&mut touch.y, 0..=941).text("Y"))
            .changed();
        if changed {
            let _ = controller.set_touch(slot, touch.contact(id));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uhid_creation_failure_distinguishes_registration_from_access() {
        use virtualgamepad::ControllerError;
        for target in [
            RealizationId::LINUX_UHID_USB,
            RealizationId::LINUX_UHID_BLUETOOTH,
        ] {
            for preflight in [
                ControllerError::AccessDenied {
                    target,
                    path: "/dev/uhid".into(),
                },
                ControllerError::MissingDeviceNode {
                    target,
                    path: "/dev/uhid".into(),
                },
            ] {
                let error = preflight;
                let missing = creation_error_message(&error, Some(false));
                assert!(missing.starts_with(&error.to_string()));
                assert!(missing.contains("registration is missing"));
                assert!(missing.contains("load the uhid module"));
                // A retry after registration must not retain the missing-module hint.
                for registration in [Some(true), None] {
                    let access = creation_error_message(&error, registration);
                    assert!(access.starts_with(&error.to_string()));
                    assert!(!access.contains("registration is missing"));
                    assert!(access.contains("device access for this login"));
                }
            }
        }
        for error in [
            ControllerError::AccessDenied {
                target: RealizationId::LINUX_UINPUT,
                path: "/dev/uinput".into(),
            },
            ControllerError::Open {
                reason: "synthetic creation failure".into(),
            },
            ControllerError::Closed,
        ] {
            assert_eq!(
                creation_error_message(&error, Some(false)),
                error.to_string()
            );
        }
    }

    #[test]
    #[ignore = "requires prepared UHID access and Linux hid-playstation; run in isolation"]
    fn gui_uhid_creation_services_and_cleans_up_real_controllers() {
        fn owned_nodes() -> Vec<std::path::PathBuf> {
            let prefix = format!(
                "HID_PHYS=virtualgamepad/uhid/dualsense/p{:x}-i",
                std::process::id()
            );
            std::fs::read_dir("/sys/bus/hid/devices")
                .unwrap()
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| {
                    std::fs::read_to_string(path.join("uevent")).is_ok_and(|text| {
                        text.lines().any(|line| {
                            line.strip_prefix(&prefix).is_some_and(|instance| {
                                !instance.is_empty()
                                    && instance.bytes().all(|byte| byte.is_ascii_hexdigit())
                            })
                        })
                    })
                })
                .collect()
        }
        fn wait_for_nodes(count: usize) {
            let deadline = std::time::Instant::now() + Duration::from_secs(5);
            loop {
                let nodes = owned_nodes();
                if nodes.len() == count && nodes.iter().all(|path| path.join("input").is_dir()) {
                    return;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "expected {count} bound devices"
                );
                thread::sleep(Duration::from_millis(10));
            }
        }
        assert!(owned_nodes().is_empty());
        let mut app = App::default();
        app.kind = Kind::DualSense;
        app.target = RealizationId::LINUX_UHID_USB;
        for count in 1..=2 {
            app.create();
            assert!(
                matches!(
                    app.lifecycle_status,
                    Some(ControllerLifecycleStatus::Created { .. })
                ),
                "{:?}",
                app.lifecycle_status
            );
            assert_eq!(app.controllers.len(), count);
            wait_for_nodes(count);
        }
        // Exercise the actual GUI workers through startup and repeated idle polls.
        thread::sleep(Duration::from_secs(1));
        for controller in &app.controllers {
            assert!(worker_failure(&controller.service_worker.as_ref().unwrap().failure).is_none());
        }
        app.remove_controller(0);
        wait_for_nodes(1);
        assert_eq!(app.controllers.len(), 1);
        app.create();
        wait_for_nodes(2);
        // App shutdown joins and closes all remaining workers.
        drop(app);
        wait_for_nodes(0);
    }

    #[test]
    fn sony_pad_conversion_covers_full_domain_and_round_trips_every_axis_value() {
        assert_eq!(dualsense_axis_to_pad(0), i16::MIN);
        assert_eq!(dualsense_axis_to_pad(128), 0);
        assert_eq!(dualsense_axis_to_pad(255), i16::MAX);
        for value in 0..=255 {
            assert_eq!(dualsense_axis_from_pad(dualsense_axis_to_pad(value)), value);
        }
        let mut previous = 0;
        for value in i16::MIN..=i16::MAX {
            let mapped = dualsense_axis_from_pad(value);
            assert!(mapped >= previous);
            previous = mapped;
        }
        for (fraction, expected) in [
            (-2.0, i16::MIN),
            (-1.0, i16::MIN),
            (0.0, 0),
            (1.0, i16::MAX),
            (2.0, i16::MAX),
        ] {
            assert_eq!(pad_axis_from_fraction(fraction), expected);
        }
    }

    #[test]
    fn printed_face_labels_preserve_spatial_nintendo_and_sony_layouts() {
        assert_eq!(
            face_labels(Kind::SwitchPro),
            ["B (South)", "A (East)", "Y (West)", "X (North)"]
        );
        assert_eq!(face_labels(Kind::DualShock4), face_labels(Kind::DualSense));
        assert_eq!(face_labels(Kind::Xbox360)[0], "A (South)");
    }

    #[test]
    fn lab_session_ids_can_repeat_and_advance_without_overflow() {
        for id in [0, 7, 65543, u64::MAX] {
            assert_eq!(following_session(id, false), id);
        }
        assert_eq!(following_session(7, true), 8);
        assert_eq!(following_session(u64::MAX, true), 0);
    }

    #[test]
    fn lab_metrics_and_record_preserve_measurement_scope() {
        let start = Instant::now();
        let mut metrics = ServiceMetrics::default();
        for delta in [0, 4, 15, 19] {
            metrics.record(start + Duration::from_millis(delta));
        }
        metrics.omit(4);
        metrics.omit(8);
        assert_eq!(metrics.cycles, 4);
        assert_eq!(metrics.max_gap, Duration::from_millis(11));
        assert_eq!(metrics.omitted_logs, 12);
        let record = lab_record(
            "Synthetic lab controller",
            LabOptions {
                session: 65543,
                target: RealizationId::LINUX_UHID_USB,
            },
            &metrics,
            "Reference disconnected; synthetic test",
            &ConsumerNotes {
                build: "synthetic build".into(),
                backend: "fake backend".into(),
                mapping: "fake mapping".into(),
            },
            "Closed; cleanup failed: synthetic",
        );
        assert!(record.starts_with("Virtualgamepad manual lab record v3"));
        assert!(record.contains("Consumer build: synthetic build"));
        assert!(record.contains("Consumer backend: fake backend"));
        assert!(record.contains("Consumer mapping: fake mapping"));
        assert!(record.contains("cleanup failed: synthetic"));
        assert!(record.contains("Lab correlation: 65543"));
        assert!(record.contains("Maximum observed service gap (us): 11000"));
        assert!(record.contains("acceptance: not established"));
        assert!(record.ends_with("Reference disconnected; synthetic test"));
    }

    #[derive(Default)]
    struct FakeService {
        refreshed: usize,
        serviced: usize,
        closed: usize,
        fail: bool,
        deadline: Option<Duration>,
        progress: Option<mpsc::Sender<usize>>,
        desired: Vec<bool>,
        committed: Vec<Vec<bool>>,
    }
    impl ServicedController for FakeService {
        fn neutralize(&mut self) -> Result<(), String> {
            if self.closed != 0 {
                return Err("closed".into());
            }
            self.desired.push(false);
            Ok(())
        }
        fn commit_edits(&mut self) -> Result<(), String> {
            self.committed.push(self.desired.clone());
            Ok(())
        }
        fn refresh(&mut self) -> Result<(), String> {
            self.refreshed += 1;
            Ok(())
        }
        fn service(
            &mut self,
            log: &mut Vec<String>,
            indicators: &mut ReverseIndicators,
        ) -> Result<(), String> {
            self.serviced += 1;
            log.push(format!("event {}", self.serviced));
            indicators.led = Some([1, 2, 3]);
            if let Some(progress) = &self.progress {
                let _ = progress.send(self.serviced);
            }
            if self.fail {
                Err("injected service failure".into())
            } else {
                Ok(())
            }
        }
        fn deadline(&self) -> Option<Duration> {
            self.deadline
        }
        fn close(&mut self) {
            self.closed += 1;
        }
    }

    fn fake_edit(pressed: bool) -> Command<FakeService> {
        Box::new(move |controller| {
            controller.desired.push(pressed);
            Ok(())
        })
    }

    #[test]
    fn release_input_uses_acknowledged_queue_and_preserves_prior_press() {
        let (sender, receiver) = mpsc::sync_channel(1);
        let mut progress = EditProgress::default();
        let mut controller = FakeService::default();
        progress.submit(&sender, vec![fake_edit(true)]).unwrap();
        assert!(progress.submit(&sender, vec![release_inputs()]).is_err());
        let (seq, edits) = receiver.try_recv().unwrap();
        controller.apply(edits).unwrap();
        assert!(progress.observe(seq));
        progress.submit(&sender, vec![release_inputs()]).unwrap();
        let (seq, edits) = receiver.try_recv().unwrap();
        controller.apply(edits).unwrap();
        assert!(progress.observe(seq));
        assert_eq!(controller.committed, vec![vec![true], vec![true, false]]);
        controller.close();
        assert!(controller.apply(vec![release_inputs()]).is_err());
    }

    #[test]
    fn unexpected_worker_exit_is_visible_without_a_failure_message() {
        let (sender, receiver) = mpsc::channel();
        assert_eq!(worker_failure(&receiver), None);
        sender.send("injected failure".into()).unwrap();
        assert_eq!(
            worker_failure(&receiver).as_deref(),
            Some("injected failure")
        );
        drop(sender);
        assert_eq!(
            worker_failure(&receiver).as_deref(),
            Some("controller worker exited unexpectedly")
        );
    }

    #[test]
    fn edit_progress_preserves_order_and_rejects_full_disconnected_or_stale_updates() {
        let (sender, receiver) = mpsc::sync_channel(1);
        let mut progress = EditProgress::default();
        progress.submit(&sender, vec![fake_edit(true)]).unwrap();
        assert!(!progress.ready());
        assert!(!progress.observe(0));
        assert!(progress.submit(&sender, vec![fake_edit(false)]).is_err());
        let (first, edits) = receiver.try_recv().unwrap();
        let mut controller = FakeService::default();
        controller.apply(edits).unwrap();
        assert!(progress.observe(first));
        progress.submit(&sender, vec![fake_edit(false)]).unwrap();
        let (second, edits) = receiver.try_recv().unwrap();
        controller.apply(edits).unwrap();
        assert!(progress.observe(second));
        assert_eq!(controller.committed, vec![vec![true], vec![true, false]]);

        sender.try_send((99, vec![fake_edit(true)])).unwrap();
        assert!(progress.submit(&sender, vec![fake_edit(true)]).is_err());
        assert_eq!(progress.submitted, second);
        drop(receiver);
        assert!(progress.submit(&sender, vec![fake_edit(false)]).is_err());
        progress.submitted = u64::MAX;
        progress.applied = u64::MAX;
        assert!(progress.submit(&sender, vec![fake_edit(false)]).is_err());
    }

    #[test]
    fn rejected_edit_batch_cannot_commit_partial_state_or_run_following_edits() {
        let mut controller = FakeService::default();
        let edits: Vec<Command<FakeService>> = vec![
            fake_edit(true),
            Box::new(|_| Err("invalid native value".into())),
            fake_edit(false),
        ];
        assert!(controller.apply(edits).is_err());
        assert_eq!(controller.desired, vec![true]);
        assert!(controller.committed.is_empty());
        let oversized = (0..=editor::EDIT_LIMIT).map(|_| fake_edit(false)).collect();
        assert!(controller.apply(oversized).is_err());
        assert_eq!(controller.desired, vec![true]);
    }

    #[test]
    fn edit_failure_closes_worker_without_display_consumption() {
        let worker = spawn_service_worker(FakeService::default());
        worker
            .edits
            .try_send((1, vec![Box::new(|_| Err("edit rejected".into()))]))
            .unwrap();
        assert_eq!(
            worker.failure.recv_timeout(Duration::from_secs(2)).unwrap(),
            "edit rejected"
        );
        let controller = worker.stop().unwrap();
        assert_eq!(controller.closed, 1);
        assert!(controller.committed.is_empty());
    }

    #[test]
    fn stop_discards_queued_input_before_another_service_cycle() {
        let worker = spawn_service_worker(FakeService::default());
        let (entered, waiting) = mpsc::channel();
        let (release, resume) = mpsc::channel();
        worker
            .edits
            .try_send((
                1,
                vec![Box::new(move |c| {
                    entered.send(()).unwrap();
                    resume.recv_timeout(Duration::from_secs(2)).unwrap();
                    c.desired.push(true);
                    Ok(())
                })],
            ))
            .unwrap();
        waiting.recv_timeout(Duration::from_secs(2)).unwrap();
        worker.edits.try_send((2, vec![fake_edit(false)])).unwrap();
        worker.stop.send(()).unwrap();
        release.send(()).unwrap();
        let controller = worker.stop().unwrap();
        assert_eq!(controller.desired, vec![true]);
        assert_eq!(controller.closed, 1);
    }

    #[test]
    fn independent_service_cycles_preserve_motion_cadence_and_short_deadlines() {
        let mut controller = FakeService {
            deadline: Some(Duration::from_micros(500)),
            ..Default::default()
        };
        let mut next_motion = Duration::ZERO;
        let mut logs = Vec::new();
        let mut indicators = ReverseIndicators::default();
        for now in [0, 500, 1000, 3500, 4000] {
            let delay = service_cycle(
                &mut controller,
                Duration::from_micros(now),
                &mut next_motion,
                &mut logs,
                &mut indicators,
            )
            .unwrap();
            assert_eq!(delay, Duration::from_micros(500));
        }
        assert_eq!(controller.serviced, 5);
        assert_eq!(controller.refreshed, 2);
        controller.deadline = Some(Duration::ZERO);
        assert_eq!(
            service_cycle(
                &mut controller,
                Duration::from_micros(4001),
                &mut next_motion,
                &mut logs,
                &mut indicators
            )
            .unwrap(),
            Duration::ZERO
        );
        assert_eq!(controller.refreshed, 2);
    }

    #[test]
    fn display_backlog_is_bounded_and_retains_latest_indicators() {
        let mut display = WorkerDisplay::default();
        let indicators = ReverseIndicators {
            led: Some([1, 2, 3]),
            ..Default::default()
        };
        for n in 0..1000 {
            publish_display(&mut display, vec![n.to_string()], &indicators);
        }
        assert_eq!(display.logs.len(), OUTPUT_LOG_LIMIT);
        assert_eq!(display.logs.first().unwrap(), "800");
        assert_eq!(display.logs.last().unwrap(), "999");
        assert_eq!(display.indicators.led, Some([1, 2, 3]));
    }

    #[test]
    fn worker_services_without_ui_and_stops_while_display_is_locked() {
        let (sender, receiver) = mpsc::channel();
        let controller = FakeService {
            progress: Some(sender),
            ..Default::default()
        };
        let worker = spawn_service_worker(controller);
        let display = Arc::clone(&worker.display);
        let guard = display.lock().unwrap();
        for _ in 0..3 {
            receiver.recv_timeout(Duration::from_secs(2)).unwrap();
        }
        let state = worker.stop().unwrap();
        assert!(state.serviced >= 3);
        assert_eq!(state.closed, 1);
        drop(guard);
    }

    #[test]
    fn removing_one_worker_preserves_another_workers_service() {
        let first = FakeService::default();
        let (sender, receiver) = mpsc::channel();
        let second = FakeService {
            progress: Some(sender),
            ..Default::default()
        };
        let first_worker = spawn_service_worker(first);
        let second_worker = spawn_service_worker(second);
        receiver.recv_timeout(Duration::from_secs(2)).unwrap();
        let first = first_worker.stop().unwrap();
        let count = receiver.recv_timeout(Duration::from_secs(2)).unwrap();
        loop {
            if receiver.recv_timeout(Duration::from_secs(2)).unwrap() > count {
                break;
            }
        }
        assert_eq!(first.closed, 1);
        let second = second_worker.stop().unwrap();
        assert_eq!(second.closed, 1);
    }

    #[test]
    fn worker_failure_closes_without_waiting_for_ui_consumption() {
        let controller = FakeService {
            fail: true,
            ..Default::default()
        };
        let worker = spawn_service_worker(controller);
        assert_eq!(
            worker.failure.recv_timeout(Duration::from_secs(2)).unwrap(),
            "injected service failure"
        );
        let state = worker.stop().unwrap();
        assert_eq!(state.serviced, 1);
        assert_eq!(state.closed, 1);
    }

    #[test]
    fn repaint_respects_immediate_and_sub_frame_service_deadlines() {
        for count in [1, 8, 300] {
            for deadline in [
                Duration::ZERO,
                Duration::from_micros(500),
                Duration::from_millis(3),
            ] {
                assert_eq!(service_repaint_interval(count, Some(deadline)), deadline);
            }
            assert_eq!(
                service_repaint_interval(count, Some(Duration::from_secs(1))),
                Duration::from_millis(4)
            );
            assert_eq!(
                service_repaint_interval(count, None),
                Duration::from_millis(4)
            );
        }
        assert_eq!(service_repaint_interval(0, None), Duration::from_millis(50));
    }

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
    fn quick_button_click_is_held_for_one_report_before_release() {
        assert_eq!(next_hold_state(false, false, true), Some(true));
        assert_eq!(next_hold_state(true, false, false), Some(false));
        assert_eq!(next_hold_state(false, false, false), None);
        assert_eq!(next_hold_state(true, true, false), None);
    }

    #[test]
    fn second_touch_latches_its_coordinates_while_inactive() {
        let mut touch = LatchedTouch {
            active: true,
            x: 123,
            y: 456,
        };
        assert_eq!(
            touch.contact(1),
            Some(DualSenseTouchContact::new(1, 123, 456).expect("bounded contact"))
        );
        touch.active = false;
        assert_eq!(touch.contact(1), None);
        assert_eq!((touch.x, touch.y), (123, 456));
        touch.active = true;
        assert_eq!(
            touch.contact(1),
            Some(DualSenseTouchContact::new(1, 123, 456).expect("bounded contact"))
        );
    }

    #[test]
    fn dualsense_motion_refresh_is_available_for_uhid_and_dummy_hcd() {
        assert!(dualsense_motion_target(RealizationId::LINUX_UHID_USB));
        assert!(dualsense_motion_target(
            RealizationId::LINUX_DUMMY_HCD_USB_HID
        ));
        assert!(!dualsense_motion_target(RealizationId::LINUX_UINPUT));
        assert_eq!(
            dualsense_motion_target_label(RealizationId::LINUX_UHID_USB),
            "UHID motion report"
        );
        assert_eq!(
            dualsense_motion_target_label(RealizationId::LINUX_DUMMY_HCD_USB_HID),
            "DummyHcd USB motion report"
        );
    }

    #[test]
    fn motion_refresh_has_the_same_target_contract_for_all_imu_controllers() {
        assert!(motion_refresh_target(RealizationId::LINUX_UHID_USB));
        assert!(motion_refresh_target(
            RealizationId::LINUX_DUMMY_HCD_USB_HID
        ));
        assert!(!motion_refresh_target(RealizationId::LINUX_UINPUT));
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
    fn effect_upload_is_not_playback_and_stop_clears_activity() {
        use virtualgamepad::{ForceFeedbackEffect, ForceFeedbackEvent, RumbleEffect};
        let mut indicators = ReverseIndicators::default();
        let effect = RumbleEffect {
            id: 0,
            strong: 1,
            weak: 2,
            length_ms: 100,
            delay_ms: 0,
            trigger_button: 0,
            trigger_interval_ms: 0,
        };
        indicators.apply_force_feedback(ForceFeedbackEvent::Uploaded {
            request_id: 7,
            effect: ForceFeedbackEffect::Rumble(effect),
            status: 0,
        });
        assert!(!indicators.rumble_active);
        assert!(indicators.rumble_until.is_none());
        indicators.apply_force_feedback(ForceFeedbackEvent::Playback {
            effect,
            repetitions: 1,
        });
        assert!(indicators.rumble_until.is_some());
        indicators.apply_force_feedback(ForceFeedbackEvent::Playback {
            effect,
            repetitions: 0,
        });
        assert!(!indicators.rumble_active);
        assert!(indicators.rumble_until.is_none());
    }

    #[test]
    fn ds4_rgb_indicator_updates_and_survives_rumble_only_output() {
        let mut indicators = ReverseIndicators::default();
        indicators.apply_hid_output(Some(0), Some(0), Some([32, 64, 128]), None);
        assert_eq!(indicators.led, Some([32, 64, 128]));
        indicators.apply_hid_output(Some(64), Some(128), None, None);
        assert_eq!(indicators.led, Some([32, 64, 128]));
        assert!(indicators.rumble_active);
    }

    #[test]
    fn partial_hid_updates_preserve_unmentioned_motors() {
        let mut indicators = ReverseIndicators::default();
        indicators.apply_hid_output(Some(64), Some(128), None, None);
        let started = indicators.rumble_started;
        indicators.apply_hid_output(None, None, Some([1, 2, 3]), Some(true));
        assert!(
            indicators.rumble_active,
            "LED-only update must preserve rumble"
        );
        assert_eq!(indicators.rumble_started, started);
        indicators.apply_hid_output(Some(0), None, None, None);
        assert!(indicators.rumble_active, "left motor remains active");
        indicators.apply_hid_output(None, Some(0), None, None);
        assert!(!indicators.rumble_active);
        assert_eq!(indicators.led, Some([1, 2, 3]));
        assert_eq!(indicators.mute_led, Some(true));
    }

    #[test]
    fn rumble_only_dualsense_output_preserves_prior_led_indicators() {
        let mut indicators = ReverseIndicators {
            led: Some([0x11, 0x22, 0x33]),
            mute_led: Some(true),
            ..ReverseIndicators::default()
        };
        indicators.apply_hid_output(Some(0x40), Some(0x20), None, None);
        assert_eq!(indicators.led, Some([0x11, 0x22, 0x33]));
        assert_eq!(indicators.mute_led, Some(true));
        assert!(indicators.rumble_active);
    }
}
