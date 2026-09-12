mod editor;
use editor::{
    Command, ControllerView, DualSenseEditor, DualShock4Editor, SwitchProEditor, Xbox360Editor,
};
use eframe::egui::{self, Button, Color32, Pos2, Sense, Stroke, Vec2};
use std::{
    cmp::Ordering,
    collections::VecDeque,
    env,
    fmt::Write as _,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use virtualgamepad::ControllerSurfaceInfo;
use virtualgamepad::{
    BatteryLevel, BatteryState, ControllerStatus, DigitalControlUpdate, DpadDirection,
    DualSenseAxis, DualSenseControl, DualSenseController, DualSenseHidOutput, DualSenseOutputEvent,
    DualSenseTouchContact, DualSenseTrigger, DualShock4Axis, DualShock4Control,
    DualShock4Controller, DualShock4HidOutput, DualShock4MotionSample, DualShock4TouchContact,
    DualShock4TouchSlot, DualShock4Trigger, FaceButton, MotionSample, RealizationId, SwitchProAxis,
    SwitchProControl, SwitchProController, SwitchProMotionSample, TouchSlot, Xbox360Axis,
    Xbox360Control, Xbox360Controller, Xbox360OutputEvent, Xbox360Trigger, create_dualsense,
    create_dualshock4, create_switch_pro, create_xbox360,
};

// Controller IDs are demo-local labels, never library/session identity.
#[derive(Clone, Copy)]
struct ControllerOptions {
    target: RealizationId,
    id: u64,
}

const OUTPUT_LOG_LIMIT: usize = 200;
const CONTROLLER_ID_WIDTH: usize = 3;
const CONTROLLER_NAME_MAX_CHARS: usize = 64;
const DUALSENSE_MOTION_INTERVAL: Duration = Duration::from_millis(4);
const GUI_REPAINT_INTERVAL: Duration = Duration::from_millis(16);
const IDLE_REPAINT_INTERVAL: Duration = Duration::from_millis(50);
const SIDEBAR_WIDTH: f32 = 200.0;
const DIAGNOSTIC_LOG_LINE_COUNT: f32 = 5.0;
const DIAGNOSTIC_LOG_TOP_MARGIN: i8 = 4;
const HEALTH_BOTTOM_PADDING: f32 = 4.0;
const NAME_INPUT_HEIGHT: f32 = 22.0;
const CREATE_COUNT_SPINBOX_WIDTH: f32 = 58.0;
const CREATE_BUTTON_FILL: Color32 = Color32::from_rgb(92, 151, 183);
const CREATE_BUTTON_TEXT: Color32 = Color32::from_rgb(245, 250, 255);
const SELECTED_CONTROLLER_TEXT: Color32 = Color32::from_rgb(235, 250, 255);
const CONTROLLER_ROW_HEIGHT: f32 = NAME_INPUT_HEIGHT;
const CONTROLLER_NUMBER_WIDTH: f32 = 16.0;
const CONTROLLER_DELETE_WIDTH: f32 = CONTROLLER_ROW_HEIGHT;
const ADVANCED_OPTIONS_BODY_HEIGHT: f32 = CONTROLLER_ROW_HEIGHT * 6.0;
const CONTROLLER_LIST_MIN_HEIGHT: f32 = CONTROLLER_ROW_HEIGHT * 4.0;
const CONTROLLER_LIST_FRAME_VERTICAL_MARGIN: f32 = 8.0;
const STATE_ROW_LABEL_WIDTH: f32 = 64.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ControllerLabelMode {
    AssignedName,
    InternalIdentifier,
}

struct DiagnosticLogEntry {
    message: String,
    success: bool,
}

impl ControllerLabelMode {
    const fn label(self) -> &'static str {
        match self {
            Self::AssignedName => "Name",
            Self::InternalIdentifier => "Identifier",
        }
    }
}

const fn backend_status_is_healthy(status: ControllerStatus) -> bool {
    matches!(status, ControllerStatus::Open)
}

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
        GUI_REPAINT_INTERVAL
    }
}

fn diagnostic_log_height(line_height: f32) -> f32 {
    (line_height * DIAGNOSTIC_LOG_LINE_COUNT) + f32::from(DIAGNOSTIC_LOG_TOP_MARGIN)
}

fn diagnostic_log_scroll_height(log_height: f32) -> f32 {
    (log_height - f32::from(DIAGNOSTIC_LOG_TOP_MARGIN)).max(0.0)
}

fn diagnostic_log_top_padding(
    viewport_height: f32,
    line_height: f32,
    entry_count: usize,
    line_spacing: f32,
) -> f32 {
    let entry_count = f32::from(
        u8::try_from(entry_count.clamp(1, OUTPUT_LOG_LIMIT))
            .expect("diagnostic log limit fits in u8"),
    );
    let content_height = (line_height * entry_count) + (line_spacing * (entry_count - 1.0));
    (viewport_height - content_height).max(0.0)
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct SidebarLayoutBudget {
    controller_list: f32,
    diagnostic_log: f32,
    footer: f32,
}

fn sidebar_layout_budget(
    available_height: f32,
    footer_controls_height: f32,
    diagnostic_log_height: f32,
    section_spacing: f32,
) -> SidebarLayoutBudget {
    let available_height = available_height.max(0.0);
    let footer_height = diagnostic_log_height + footer_controls_height;
    let controller_list_height = (available_height
        - footer_height
        - CONTROLLER_LIST_FRAME_VERTICAL_MARGIN
        - (section_spacing * 2.0))
        .max(CONTROLLER_LIST_MIN_HEIGHT);
    SidebarLayoutBudget {
        controller_list: controller_list_height,
        diagnostic_log: diagnostic_log_height,
        footer: footer_height,
    }
}

const fn advanced_options_available(_target: RealizationId) -> bool {
    true
}

fn service_repaint_interval(controller_count: usize, next_service: Option<Duration>) -> Duration {
    let fallback = repaint_interval(controller_count);
    next_service.map_or(fallback, |deadline| deadline.min(fallback))
}

const fn motion_worker_interval() -> Duration {
    DUALSENSE_MOTION_INTERVAL
}

fn controller_tab_indices(controller_count: usize) -> std::ops::Range<usize> {
    0..controller_count
}

fn selection_after_removal(
    remaining_count: usize,
    removed_index: usize,
    selected_index: Option<usize>,
) -> Option<usize> {
    if remaining_count == 0 {
        None
    } else {
        selected_index.map(|selected_index| match selected_index.cmp(&removed_index) {
            Ordering::Equal => removed_index.min(remaining_count - 1),
            Ordering::Greater => selected_index - 1,
            Ordering::Less => selected_index,
        })
    }
}

fn selection_after_controller_click(
    selected_index: Option<usize>,
    clicked_index: usize,
    clicked: bool,
) -> Option<usize> {
    clicked.then_some(clicked_index).or(selected_index)
}

fn controller_removal_after_delete_click(index: usize, clicked: bool) -> Option<usize> {
    clicked.then_some(index)
}

fn successful_controller_close_message(name: &str) -> String {
    format!("Closed {name}.")
}

const fn stop_all_after_click(clicked: bool) -> bool {
    clicked
}

fn controller_row_fill(active: bool, selected_fill: Color32) -> Color32 {
    if active {
        selected_fill
    } else {
        Color32::from_gray(62)
    }
}

const fn controller_row_has_hover_outline(hovered: bool) -> bool {
    hovered
}

fn sidebar_item_text_color(hovered: bool, normal: Color32, highlighted: Color32) -> Color32 {
    if hovered { highlighted } else { normal }
}

fn controller_row_text_color(
    active: bool,
    hovered: bool,
    normal: Color32,
    highlighted: Color32,
) -> Color32 {
    if active {
        SELECTED_CONTROLLER_TEXT
    } else {
        sidebar_item_text_color(hovered, normal, highlighted)
    }
}

fn sidebar_choice_chip(
    ui: &mut egui::Ui,
    label: &'static str,
    selected: bool,
    width: f32,
) -> egui::Response {
    let highlighted = ui.visuals().strong_text_color();
    ui.scope(|ui| {
        ui.visuals_mut().widgets.hovered.fg_stroke.color = highlighted;
        if selected {
            ui.visuals_mut().widgets.inactive.fg_stroke.color = highlighted;
        }
        ui.add_sized([width, 22.0], egui::Button::new(label).selected(selected))
    })
    .inner
}

fn next_available_name(kind: Kind, existing_names: impl Iterator<Item = String>) -> String {
    let existing_names: std::collections::HashSet<String> = existing_names.collect();
    (0..=existing_names.len())
        .map(|number| format!("{} {number}", kind.label()))
        .find(|name| !existing_names.contains(name))
        .expect("unbounded controller name search must find an available name")
}

fn sanitized_controller_name(draft: &str) -> Result<String, &'static str> {
    let name = draft.trim();
    if name.is_empty() {
        return Err("Name cannot be empty");
    }
    if name.chars().count() > CONTROLLER_NAME_MAX_CHARS {
        return Err("Name must be 64 characters or fewer");
    }
    if name.chars().any(char::is_control) {
        return Err("Name cannot contain control characters");
    }
    Ok(name.to_owned())
}

fn apply_controller_name(controller: &mut NamedController) {
    match sanitized_controller_name(&controller.name_draft) {
        Ok(name) => {
            controller.name_draft.clone_from(&name);
            controller.name = name;
            controller.name_error = None;
        }
        Err(error) => controller.name_error = Some(error.into()),
    }
}

fn truncate_identifier(identifier: &str, max_chars: usize) -> String {
    let mut chars = identifier.chars();
    let visible: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{visible}…")
    } else {
        visible
    }
}

fn controller_identifier(controller: &NamedController) -> String {
    truncate_identifier(
        &controller_id(
            controller.options.id,
            controller.options.target,
            controller.kind,
        ),
        28,
    )
}

fn controller_id(number: u64, target: RealizationId, kind: Kind) -> String {
    format!(
        "{:0width$}-{}-{}",
        number % 1_000,
        target_identifier_abbreviation(target),
        kind.identifier_abbreviation(),
        width = CONTROLLER_ID_WIDTH,
    )
}

fn target_identifier_abbreviation(target: RealizationId) -> &'static str {
    match target {
        RealizationId::LINUX_UINPUT => "UIN",
        RealizationId::LINUX_UHID_USB => "HID",
        RealizationId::LINUX_DUMMY_HCD_USB_HID => "USB",
        _ => "UNK",
    }
}

struct TargetHelp {
    title: &'static str,
    body: &'static str,
}

fn target_help(target: RealizationId) -> Option<TargetHelp> {
    match target {
        RealizationId::LINUX_DUMMY_HCD_USB_HID => Some(TargetHelp {
            title: "Experimental USB gadget",
            body: "Requires the privileged broker and prepared dummy_hcd resources. Complete Gate G host setup before validation. This demo surface is for research and test use only.",
        }),
        _ => None,
    }
}

fn state_dump_directory() -> PathBuf {
    env::var_os("XDG_STATE_HOME")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .unwrap_or_else(env::temp_dir)
        .join("virtualgamepad")
}

fn state_dump_path(now: SystemTime) -> Result<PathBuf, String> {
    let timestamp = now
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock is before the Unix epoch: {error}"))?
        .as_millis();
    Ok(state_dump_directory().join(format!("demo-state-{timestamp}-{}.log", std::process::id())))
}

fn requested_create_count(name: &str, count: u32) -> u32 {
    if name.trim().is_empty() {
        count.max(1)
    } else {
        1
    }
}

fn step_create_count(count: u32, increment: bool) -> u32 {
    if increment {
        count.saturating_add(1)
    } else {
        count.saturating_sub(1).max(1)
    }
}

fn spinbox_arrow_rects(rect: egui::Rect, arrow_width: f32) -> (egui::Rect, egui::Rect) {
    let arrow_left = rect.right() - arrow_width;
    (
        egui::Rect::from_min_max(
            Pos2::new(arrow_left, rect.top()),
            Pos2::new(rect.right(), rect.center().y),
        ),
        egui::Rect::from_min_max(Pos2::new(arrow_left, rect.center().y), rect.right_bottom()),
    )
}

fn paint_spinbox_arrow(ui: &egui::Ui, rect: egui::Rect, points_up: bool, hovered: bool) {
    let center = rect.center();
    let inset = 3.0;
    let half_width = 3.0;
    let points = if points_up {
        vec![
            Pos2::new(center.x, rect.top() + inset),
            Pos2::new(center.x - half_width, rect.bottom() - inset),
            Pos2::new(center.x + half_width, rect.bottom() - inset),
        ]
    } else {
        vec![
            Pos2::new(center.x - half_width, rect.top() + inset),
            Pos2::new(center.x + half_width, rect.top() + inset),
            Pos2::new(center.x, rect.bottom() - inset),
        ]
    };
    let color = if hovered {
        ui.visuals().strong_text_color()
    } else {
        ui.visuals().weak_text_color()
    };
    ui.painter()
        .add(egui::Shape::convex_polygon(points, color, Stroke::NONE));
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DisclosureDirection {
    Right,
    Down,
}

const fn advanced_disclosure_direction(expanded: bool) -> DisclosureDirection {
    if expanded {
        DisclosureDirection::Down
    } else {
        DisclosureDirection::Right
    }
}

fn paint_advanced_disclosure_arrow(
    ui: &egui::Ui,
    rect: egui::Rect,
    direction: DisclosureDirection,
    hovered: bool,
) {
    let center = rect.center();
    let inset = 5.0;
    let half_width = 3.0;
    let points = match direction {
        DisclosureDirection::Right => vec![
            Pos2::new(rect.left() + inset, center.y - half_width),
            Pos2::new(rect.left() + inset, center.y + half_width),
            Pos2::new(rect.right() - inset, center.y),
        ],
        DisclosureDirection::Down => vec![
            Pos2::new(center.x - half_width, rect.top() + inset),
            Pos2::new(center.x + half_width, rect.top() + inset),
            Pos2::new(center.x, rect.bottom() - inset),
        ],
    };
    let color = if hovered {
        ui.visuals().strong_text_color()
    } else {
        ui.visuals().weak_text_color()
    };
    ui.painter()
        .add(egui::Shape::convex_polygon(points, color, Stroke::NONE));
}

fn create_count_spinbox(ui: &mut egui::Ui, value: &mut u32) -> egui::Response {
    const ARROW_WIDTH: f32 = 16.0;
    const SPINBOX_HEIGHT: f32 = 22.0;
    let id = ui.make_persistent_id("create_count_spinbox");
    let mut text = ui.data_mut(|data| {
        data.get_temp::<String>(id)
            .unwrap_or_else(|| value.to_string())
    });
    let text_response = ui.add_sized(
        [CREATE_COUNT_SPINBOX_WIDTH, SPINBOX_HEIGHT],
        egui::TextEdit::singleline(&mut text)
            .desired_width(CREATE_COUNT_SPINBOX_WIDTH)
            .horizontal_align(egui::Align::RIGHT)
            .vertical_align(egui::Align::Center)
            .margin(egui::Margin {
                left: 4,
                right: 20,
                top: 2,
                bottom: 2,
            })
            .id(id),
    );
    if text_response.changed() {
        if let Ok(parsed) = text.trim().parse::<u32>() {
            *value = parsed.max(1);
            if parsed == 0 {
                text = value.to_string();
            }
        }
    }
    if text_response.lost_focus() {
        text = value.to_string();
    }
    let (increment_rect, decrement_rect) = spinbox_arrow_rects(text_response.rect, ARROW_WIDTH);
    let increment_response = ui.interact(increment_rect, id.with("increment"), Sense::click());
    let decrement_response = ui.interact(decrement_rect, id.with("decrement"), Sense::click());
    paint_spinbox_arrow(ui, increment_rect, true, increment_response.hovered());
    paint_spinbox_arrow(ui, decrement_rect, false, decrement_response.hovered());
    if increment_response.clicked() {
        *value = step_create_count(*value, true);
        text = value.to_string();
    }
    if decrement_response.clicked() {
        *value = step_create_count(*value, false);
        text = value.to_string();
    }
    let response = text_response
        .union(increment_response)
        .union(decrement_response);
    ui.data_mut(|data| data.insert_temp(id, text));
    response
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

    const fn identifier_abbreviation(self) -> &'static str {
        match self {
            Self::Xbox360 => "XB360",
            Self::DualSense => "DUALSENSE",
            Self::DualShock4 => "DS4",
            Self::SwitchPro => "SWITCHPRO",
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
    options: ControllerOptions,
    name: String,
    name_draft: String,
    name_error: Option<String>,
    view: ControllerView,
    edits: EditProgress,
    indicators: ReverseIndicators,
    output_log: Vec<String>,
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

#[derive(Clone)]
struct ServiceMetrics {
    cycles: u64,
    max_gap: Duration,
    omitted_logs: u64,
    last_service: Option<Instant>,
    gap_history: Arc<Mutex<VecDeque<(Instant, Duration)>>>,
}
impl Default for ServiceMetrics {
    fn default() -> Self {
        Self {
            cycles: 0,
            max_gap: Duration::ZERO,
            omitted_logs: 0,
            last_service: None,
            gap_history: Arc::new(Mutex::new(VecDeque::new())),
        }
    }
}
impl ServiceMetrics {
    fn record(&mut self, now: Instant) {
        if let Some(previous) = self.last_service {
            let gap = now.saturating_duration_since(previous);
            self.max_gap = self.max_gap.max(gap);
            if let Ok(mut history) = self.gap_history.lock() {
                history.push_back((now, gap));
            }
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

fn service_gap_percentiles(metrics: &ServiceMetrics, period_seconds: u32) -> Option<[Duration; 3]> {
    let cutoff = Instant::now().checked_sub(Duration::from_secs(u64::from(period_seconds)));
    let mut gaps: Vec<Duration> = metrics
        .gap_history
        .lock()
        .ok()?
        .iter()
        .filter(|(at, _)| period_seconds == 0 || cutoff.is_none_or(|cutoff| *at >= cutoff))
        .map(|(_, gap)| *gap)
        .collect();
    if gaps.is_empty() {
        return None;
    }
    gaps.sort_unstable();
    let percentile = |numerator: usize, denominator: usize| {
        let index = (gaps.len() * numerator)
            .div_ceil(denominator)
            .saturating_sub(1);
        gaps[index]
    };
    Some([
        percentile(90, 100),
        percentile(99, 100),
        percentile(999, 1_000),
    ])
}

#[derive(Default)]
struct WorkerDisplay {
    snapshot: Option<ControllerView>,
    applied: u64,
    logs: Vec<String>,
    metrics: ServiceMetrics,
    indicators: ReverseIndicators,
    backend_healthy: Option<bool>,
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
    fn backend_healthy(&mut self) -> bool {
        true
    }

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
    fn backend_healthy(&mut self) -> bool {
        Controller::backend_healthy(self)
    }

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

fn label_output_logs(label: &str, logs: &mut [String]) {
    for log in logs {
        *log = format!("{label}: {log}");
    }
}

#[cfg(test)]
fn spawn_service_worker<C: ServicedController + 'static>(controller: C) -> ServiceWorker<C> {
    spawn_service_worker_with_label(controller, "Controller".into())
}

fn spawn_service_worker_with_label<C: ServicedController + 'static>(
    mut controller: C,
    log_label: String,
) -> ServiceWorker<C> {
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
            label_output_logs(&log_label, &mut logs);
            let backend_healthy = controller.backend_healthy();
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
                display.backend_healthy = Some(backend_healthy);
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
    rumble_seen: bool,
    hid_motors: [u8; 2],
    rumble_started: Option<Instant>,
}
impl ReverseIndicators {
    fn rumble_pulse(&mut self) {
        self.rumble_until = Some(Instant::now() + Duration::from_millis(750));
    }
    fn set_rumble(&mut self, active: bool) {
        self.rumble_seen = true;
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
            self.rumble_seen = true;
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
    fn backend_healthy(&mut self) -> bool {
        let status = match self {
            Self::Xbox(controller) => controller.diagnostics().status(),
            Self::DualSense(controller) => controller.diagnostics().status(),
            Self::DualShock4(controller) => controller.diagnostics().status(),
            Self::SwitchPro(controller) => controller.diagnostics().status(),
        };
        backend_status_is_healthy(status)
    }

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
    const fn supports_battery_emulation(&self) -> bool {
        matches!(self, Self::Xbox(_) | Self::DualSense(_))
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
    create_count: u32,
    advanced_options_open: bool,
    next_controller_id: u64,
    polling_period_seconds: u32,
    last_cleanup: Option<String>,
    controllers: Vec<NamedController>,
    selected_controller: Option<usize>,
    controller_label_mode: ControllerLabelMode,
    diagnostic_log: Vec<DiagnosticLogEntry>,
    lifecycle_status: Option<ControllerLifecycleStatus>,
    backend_healthy: bool,
}
impl Default for App {
    fn default() -> Self {
        Self {
            kind: Kind::Xbox360,
            target: RealizationId::LINUX_UINPUT,
            name_draft: String::new(),
            create_count: 1,
            advanced_options_open: false,
            next_controller_id: 0,
            polling_period_seconds: 0,
            last_cleanup: None,
            controllers: vec![],
            selected_controller: None,
            controller_label_mode: ControllerLabelMode::AssignedName,
            diagnostic_log: Vec::new(),
            lifecycle_status: None,
            backend_healthy: true,
        }
    }
}
impl App {
    fn next_default_name(&self) -> String {
        next_available_name(
            self.kind,
            self.controllers
                .iter()
                .filter(|controller| controller.kind == self.kind)
                .map(|controller| controller.name.clone()),
        )
    }

    fn state_dump(&self) -> String {
        let mut dump = format!(
            "virtualgamepad demo state dump\nBackend health: {}\nSelected controller: {:?}\nController count: {}\n\n",
            if self.backend_healthy {
                "healthy"
            } else {
                "attention"
            },
            self.selected_controller,
            self.controllers.len(),
        );
        for (index, controller) in self.controllers.iter().enumerate() {
            let _ = writeln!(
                dump,
                "Controller {}\n  Name: {}\n  ID: {}\n  Target: {}\n  Type: {}\n  Input batch: applied {} / submitted {}\n  View diagnostics: {}",
                index + 1,
                controller.name,
                controller_identifier(controller),
                target_label(controller.options.target),
                controller.kind.label(),
                controller.edits.applied,
                controller.edits.submitted,
                controller.view.lab_details(),
            );
            if let Some(worker) = &controller.service_worker {
                if let Ok(display) = worker.display.try_lock() {
                    let _ = writeln!(
                        dump,
                        "  Service cycles: {}\n  Maximum observed service gap (us): {}\n  Omitted worker logs: {}\n  Backend healthy: {}",
                        display.metrics.cycles,
                        display.metrics.max_gap.as_micros(),
                        display.metrics.omitted_logs,
                        display.backend_healthy.unwrap_or(false),
                    );
                } else {
                    dump.push_str("  Worker display: busy\n");
                }
            }
            if !controller.output_log.is_empty() {
                dump.push_str("  Typed reverse output:\n");
                for entry in &controller.output_log {
                    let _ = writeln!(dump, "    {entry}");
                }
            }
            dump.push('\n');
        }
        if let Some(cleanup) = &self.last_cleanup {
            let _ = writeln!(dump, "Last cleanup diagnostics:\n{cleanup}\n");
        }
        dump.push_str("GUI diagnostic log:\n");
        for entry in &self.diagnostic_log {
            let _ = writeln!(
                dump,
                "[{}] {}",
                if entry.success { "success" } else { "error" },
                entry.message
            );
        }
        dump
    }

    fn write_state_dump(&mut self) {
        let result = (|| {
            let path = state_dump_path(SystemTime::now())?;
            let directory = path
                .parent()
                .ok_or_else(|| "state dump path has no parent directory".to_owned())?;
            fs::create_dir_all(directory)
                .map_err(|error| format!("could not create state-log directory: {error}"))?;
            fs::write(&path, self.state_dump())
                .map_err(|error| format!("could not write state dump: {error}"))?;
            Ok::<PathBuf, String>(path)
        })();
        match result {
            Ok(path) => self.diagnostic_log.push(DiagnosticLogEntry {
                message: format!("State dump saved to {}.", path.display()),
                success: true,
            }),
            Err(error) => self.diagnostic_log.push(DiagnosticLogEntry {
                message: format!("State dump failed: {error}"),
                success: false,
            }),
        }
    }

    fn create(&mut self) {
        let count = requested_create_count(&self.name_draft, self.create_count);
        for _ in 0..count {
            self.create_one();
            if matches!(
                self.lifecycle_status,
                Some(ControllerLifecycleStatus::CreationFailed { .. })
            ) {
                break;
            }
        }
    }

    fn create_one(&mut self) {
        let options = ControllerOptions {
            target: self.target,
            id: self.next_controller_id,
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
                let service_worker = Some(spawn_service_worker_with_label(
                    controller,
                    format!(
                        "{} · {}",
                        name,
                        controller_id(options.id, options.target, self.kind)
                    ),
                ));
                self.controllers.push(NamedController {
                    kind: self.kind,
                    options,
                    name: name.clone(),
                    name_draft: name.clone(),
                    name_error: None,
                    view,
                    edits: EditProgress::default(),
                    indicators: ReverseIndicators::default(),
                    output_log: Vec::new(),
                    service_worker,
                    second_touch: LatchedTouch::default(),
                });
                self.selected_controller = Some(self.controllers.len() - 1);
                self.name_draft.clear();
                self.next_controller_id = self.next_controller_id.wrapping_add(1);
                self.diagnostic_log.push(DiagnosticLogEntry {
                    message: format!("Created {name}."),
                    success: true,
                });
                self.lifecycle_status = Some(ControllerLifecycleStatus::Created { name });
            }
            Err(error) => {
                let error = creation_error_message(
                    &error,
                    std::path::Path::new("/sys/class/misc/uhid/dev")
                        .try_exists()
                        .ok(),
                );
                self.diagnostic_log.push(DiagnosticLogEntry {
                    message: format!("Creation failed: {error}"),
                    success: false,
                });
                self.lifecycle_status = Some(ControllerLifecycleStatus::CreationFailed { error });
            }
        }
    }

    fn remove_controller(&mut self, index: usize) {
        if index >= self.controllers.len() {
            return;
        }
        let selected_index = self.selected_controller;
        let mut removed = self.controllers.remove(index);
        let name = removed.name.clone();
        if let Some(worker) = removed.service_worker.take() {
            if let Some(mut controller) = worker.stop() {
                self.last_cleanup = Some(controller.snapshot().lab_details());
                self.diagnostic_log.push(DiagnosticLogEntry {
                    message: successful_controller_close_message(&name),
                    success: true,
                });
            } else {
                self.last_cleanup = Some(
                    "Worker exited without a returned controller; host cleanup requires verification"
                        .into(),
                );
                self.diagnostic_log.push(DiagnosticLogEntry {
                    message: format!("Close of {name} could not be verified."),
                    success: false,
                });
            }
        }
        self.selected_controller =
            selection_after_removal(self.controllers.len(), index, selected_index);
    }

    fn close_failed_controller(&mut self, index: usize, error: String) {
        let name = self.controllers[index].name.clone();
        self.diagnostic_log.push(DiagnosticLogEntry {
            message: format!("{name} closed after provider failure: {error}"),
            success: false,
        });
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
        let mut dump_state = false;
        let mut failed_controller = None;
        let mut backend_healthy = true;
        for (index, named) in self.controllers.iter_mut().enumerate() {
            if let Some(worker) = &named.service_worker {
                if let Some(error) = worker_failure(&worker.failure) {
                    backend_healthy = false;
                    failed_controller = Some((index, error));
                    break;
                }
            }
            if let Some(worker) = &named.service_worker {
                if let Ok(mut display) = worker.display.try_lock() {
                    named.output_log.append(&mut display.logs);
                    let excess = named.output_log.len().saturating_sub(OUTPUT_LOG_LIMIT);
                    named.output_log.drain(..excess);
                    named.indicators = display.indicators.clone();
                    if let Some(healthy) = display.backend_healthy {
                        backend_healthy &= healthy;
                    }
                    if display.snapshot.is_some() && named.edits.observe(display.applied) {
                        named.view = display.snapshot.take().expect("checked snapshot");
                    }
                }
            }
        }
        self.backend_healthy = backend_healthy;
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(
            "virtualgamepad Demo GUI".to_owned(),
        ));
        ctx.request_repaint_after(service_repaint_interval(self.controllers.len(), None));
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .scroll_source(egui::scroll_area::ScrollSource {
                    scroll_bar: true,
                    drag: false,
                    mouse_wheel: true,
            })
            .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(SIDEBAR_WIDTH);
                    ui.set_min_width(SIDEBAR_WIDTH);
                    ui.set_max_width(SIDEBAR_WIDTH);
                    ui.heading("Add Controller");
                    ui.add_sized([SIDEBAR_WIDTH, 1.0], egui::Separator::default());
                    egui::Grid::new("controller_creation_grid")
                        .num_columns(2)
                        .spacing([6.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Type");
                            egui::ComboBox::from_id_salt("controller_type")
                                .selected_text(self.kind.label())
                                .width(ui.available_width())
                                .show_ui(ui, |ui| {
                                    for kind in Kind::ALL {
                                        ui.selectable_value(&mut self.kind, kind, kind.label());
                                    }
                            });
                            ui.end_row();
                            ui.label("Target");
                            ui.horizontal(|ui| {
                                let help = target_help(self.target);
                                let help_width = if help.is_some() { 26.0 } else { 0.0 };
                                egui::ComboBox::from_id_salt("controller_target")
                                    .selected_text(target_label(self.target))
                                    .width((ui.available_width() - help_width).max(60.0))
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
                                        ui.selectable_value(
                                            &mut self.target,
                                            RealizationId::LINUX_DUMMY_HCD_USB_HID,
                                            target_label(RealizationId::LINUX_DUMMY_HCD_USB_HID),
                                        );
                                });
                                if let Some(help) = help {
                                    let help_response = ui.add_sized([18.0, 18.0], Button::new("!"));
                                    if help_response.hovered() {
                                        egui::Tooltip::for_widget(&help_response)
                                            .at_pointer()
                                            .show(|ui| {
                                            ui.strong(help.title);
                                            ui.label(help.body);
                                            });
                                    }
                                }
                            });
                            ui.end_row();
                        });
                    let default_name = self.next_default_name();
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        let name_is_default = self.name_draft.trim().is_empty();
                        let clear_width = 18.0;
                        let count_control_width = CREATE_COUNT_SPINBOX_WIDTH;
                        let edit_width = if name_is_default {
                            (ui.available_width()
                                - count_control_width
                                - ui.spacing().item_spacing.x)
                                .max(40.0)
                        } else {
                            ui.available_width()
                        };
                        let name_response = ui
                            .add_sized(
                                [edit_width, NAME_INPUT_HEIGHT],
                                egui::TextEdit::singleline(&mut self.name_draft)
                                    .hint_text(default_name)
                                    .desired_width(edit_width)
                                    .vertical_align(egui::Align::Center)
                                    .margin(egui::Margin {
                                        left: 4,
                                        right: 22,
                                        top: 2,
                                        bottom: 2,
                                    }),
                            )
                            .on_hover_text(
                                "Optional name. Leave empty for the automatic controller name.",
                            );
                        if !name_is_default {
                            let clear_rect = egui::Rect::from_min_max(
                                Pos2::new(name_response.rect.right() - clear_width, name_response.rect.top()),
                                name_response.rect.right_bottom(),
                            );
                            if ui
                                .put(
                                    clear_rect,
                                    Button::new("×")
                                        .frame(false)
                                        .min_size(Vec2::ZERO),
                                )
                                .on_hover_text("Clear name")
                                .clicked()
                            {
                                self.name_draft.clear();
                            }
                        }
                        if name_is_default {
                            ui.allocate_ui_with_layout(
                                Vec2::new(count_control_width, 22.0),
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    create_count_spinbox(ui, &mut self.create_count);
                                },
                            );
                        }
                    });
                    let create_clicked = ui
                        .scope(|ui| {
                            ui.visuals_mut().widgets.inactive.fg_stroke.color = CREATE_BUTTON_TEXT;
                            ui.visuals_mut().widgets.hovered.fg_stroke.color = CREATE_BUTTON_TEXT;
                            ui.add_sized(
                                [ui.available_width(), 22.0],
                                egui::Button::new("Create").fill(CREATE_BUTTON_FILL),
                            )
                        })
                        .inner
                        .clicked();
                    if create_clicked {
                        self.create();
                    }
                    let advanced_available = advanced_options_available(self.target);
                    if !advanced_available {
                        self.advanced_options_open = false;
                    }
                    let advanced_label = "Advanced options";
                    let advanced_width = ui.available_width();
                    let (advanced_rect, advanced_response) = ui.allocate_exact_size(
                        Vec2::new(advanced_width, CONTROLLER_ROW_HEIGHT),
                        if advanced_available {
                            Sense::click()
                        } else {
                            Sense::hover()
                        },
                    );
                    let advanced_fill = if advanced_available {
                        Color32::from_gray(62)
                    } else {
                        ui.visuals().widgets.noninteractive.bg_fill
                    };
                    ui.painter().rect_filled(advanced_rect, 0.0, advanced_fill);
                    if advanced_available
                        && controller_row_has_hover_outline(advanced_response.hovered())
                    {
                        ui.painter().rect_stroke(
                            advanced_rect,
                            0.0,
                            ui.visuals().widgets.hovered.bg_stroke,
                            egui::StrokeKind::Inside,
                        );
                    }
                    let advanced_hovered =
                        advanced_available && controller_row_has_hover_outline(advanced_response.hovered());
                    let advanced_highlighted =
                        advanced_available && (advanced_hovered || self.advanced_options_open);
                    ui.painter().text(
                        advanced_rect.left_center() + egui::vec2(24.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        advanced_label,
                        egui::TextStyle::Button.resolve(ui.style()),
                        sidebar_item_text_color(
                            advanced_highlighted,
                            if advanced_available {
                                ui.visuals().widgets.inactive.text_color()
                            } else {
                                ui.visuals().weak_text_color()
                            },
                            ui.visuals().strong_text_color(),
                        ),
                    );
                    let arrow_rect = egui::Rect::from_min_max(
                        advanced_rect.left_top(),
                        Pos2::new(advanced_rect.left() + 18.0, advanced_rect.bottom()),
                    );
                    paint_advanced_disclosure_arrow(
                        ui,
                        arrow_rect,
                        advanced_disclosure_direction(self.advanced_options_open),
                        advanced_available && advanced_response.hovered(),
                    );
                    if advanced_response.clicked() {
                        self.advanced_options_open = !self.advanced_options_open;
                    }
                    if self.advanced_options_open && advanced_available {
                        egui::Frame::NONE
                            .fill(Color32::from_gray(20))
                            .show(ui, |ui| {
                                ui.set_width(SIDEBAR_WIDTH);
                                egui::ScrollArea::vertical()
                                    .id_salt("advanced_options")
                                    .min_scrolled_height(ADVANCED_OPTIONS_BODY_HEIGHT)
                                    .max_height(ADVANCED_OPTIONS_BODY_HEIGHT)
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        ui.set_width(SIDEBAR_WIDTH - 8.0);
                                        ui.strong("Controller ID preview");
                                        let mut preview = controller_id(
                                            self.next_controller_id,
                                            self.target,
                                            self.kind,
                                        );
                                        let preview_width = ui.available_width();
                                        ui.add_sized(
                                            [preview_width, NAME_INPUT_HEIGHT],
                                            egui::TextEdit::singleline(&mut preview)
                                                .interactive(false)
                                                .desired_width(preview_width)
                                                .vertical_align(egui::Align::Center)
                                                .margin(egui::Margin {
                                                    left: 4,
                                                    right: 4,
                                                    top: 2,
                                                    bottom: 2,
                                                }),
                                        );
                                    });
                            });
                    }
                    ui.add_sized([SIDEBAR_WIDTH, 1.0], egui::Separator::default());
                    let controller_surface_width = SIDEBAR_WIDTH - 8.0;
                    let selector_spacing = ui.spacing().item_spacing.x;
                    let controller_content_width = controller_surface_width - selector_spacing;
                    let row_spacing = 1.0;
                    let controller_button_width = (controller_content_width
                        - CONTROLLER_NUMBER_WIDTH
                        - CONTROLLER_DELETE_WIDTH
                        - (row_spacing * 2.0))
                        .max(40.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(SIDEBAR_WIDTH, 22.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.spacing_mut().item_spacing.x = selector_spacing;
                            let chip_width = (SIDEBAR_WIDTH - selector_spacing) / 2.0;
                        if sidebar_choice_chip(
                            ui,
                            ControllerLabelMode::AssignedName.label(),
                            self.controller_label_mode == ControllerLabelMode::AssignedName,
                            chip_width,
                        )
                        .clicked()
                        {
                            self.controller_label_mode = ControllerLabelMode::AssignedName;
                        }
                        if sidebar_choice_chip(
                            ui,
                            ControllerLabelMode::InternalIdentifier.label(),
                            self.controller_label_mode == ControllerLabelMode::InternalIdentifier,
                            chip_width,
                        )
                        .clicked()
                        {
                            self.controller_label_mode = ControllerLabelMode::InternalIdentifier;
                        }
                        },
                    );
                    let footer_controls_height =
                        (CONTROLLER_ROW_HEIGHT * 2.0) + 1.0 + HEALTH_BOTTOM_PADDING;
                    let diagnostic_log_height =
                        diagnostic_log_height(ui.text_style_height(&egui::TextStyle::Body));
                    let sidebar_layout = sidebar_layout_budget(
                        ui.available_height(),
                        footer_controls_height,
                        diagnostic_log_height,
                        ui.spacing().item_spacing.y,
                    );
                    let list_height = sidebar_layout.controller_list;
                    let (controller_list_rect, _) = ui.allocate_exact_size(
                        Vec2::new(
                            SIDEBAR_WIDTH,
                            list_height + CONTROLLER_LIST_FRAME_VERTICAL_MARGIN,
                        ),
                        Sense::hover(),
                    );
                    ui.painter()
                        .rect_filled(controller_list_rect, 0.0, Color32::from_gray(20));
                    let controller_content_rect = controller_list_rect.shrink(4.0);
                    let mut list_ui = ui.new_child(
                        egui::UiBuilder::new().max_rect(controller_content_rect),
                    );
                    egui::ScrollArea::vertical()
                        .id_salt("controller_list")
                        .min_scrolled_height(list_height)
                        .max_height(list_height)
                        .auto_shrink([false, false])
                        .show(&mut list_ui, |ui| {
                            ui.set_width(controller_surface_width);
                            ui.scope(|ui| {
                                ui.spacing_mut().item_spacing.x = 1.0;
                            for index in controller_tab_indices(self.controllers.len()) {
                                let controller = &self.controllers[index];
                                let active = self.selected_controller == Some(index);
                                let label = match self.controller_label_mode {
                                    ControllerLabelMode::AssignedName => controller.name.clone(),
                                    ControllerLabelMode::InternalIdentifier => {
                                        controller_identifier(controller)
                                    }
                                };
                                ui.allocate_ui_with_layout(
                                    Vec2::new(controller_content_width, CONTROLLER_ROW_HEIGHT),
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                    ui.add_sized(
                                        [CONTROLLER_NUMBER_WIDTH, CONTROLLER_ROW_HEIGHT],
                                        egui::Label::new(format!("{}", index + 1)),
                                    );
                                    let (rect, controller_response) = ui.allocate_exact_size(
                                        Vec2::new(controller_button_width, CONTROLLER_ROW_HEIGHT),
                                        Sense::click(),
                                    );
                                    let fill = controller_row_fill(
                                        active,
                                        ui.visuals().selection.bg_fill,
                                    );
                                    ui.painter().rect_filled(rect, 0.0, fill);
                                    if controller_row_has_hover_outline(controller_response.hovered()) {
                                        ui.painter().rect_stroke(
                                            rect,
                                            0.0,
                                            ui.visuals().widgets.hovered.bg_stroke,
                                            egui::StrokeKind::Inside,
                                        );
                                    }
                                    ui.painter().text(
                                        rect.left_center() + egui::vec2(6.0, 0.0),
                                        egui::Align2::LEFT_CENTER,
                                        label,
                                        egui::TextStyle::Button.resolve(ui.style()),
                                        controller_row_text_color(
                                            active,
                                            controller_response.hovered(),
                                            ui.visuals().text_color(),
                                            ui.visuals().strong_text_color(),
                                        ),
                                    );
                                    self.selected_controller = selection_after_controller_click(
                                        self.selected_controller,
                                        index,
                                        controller_response.clicked(),
                                    );
                                    let delete_clicked = ui
                                        .add_sized(
                                            [CONTROLLER_DELETE_WIDTH, CONTROLLER_ROW_HEIGHT],
                                            egui::Button::new("×").fill(Color32::from_rgb(150, 45, 45)),
                                        )
                                        .on_hover_text("Remove controller")
                                        .clicked();
                                    if let Some(index) =
                                        controller_removal_after_delete_click(index, delete_clicked)
                                    {
                                        remove = Some(index);
                                    }
                                    },
                                );
                            }
                            });
                        });
                    let footer_height = sidebar_layout.footer;
                    let log_height = sidebar_layout.diagnostic_log;
                    let (footer_rect, _) = ui.allocate_exact_size(
                        Vec2::new(SIDEBAR_WIDTH, footer_height),
                        Sense::hover(),
                    );
                    let mut footer_ui = ui.new_child(egui::UiBuilder::new().max_rect(footer_rect));
                    footer_ui.spacing_mut().item_spacing = Vec2::ZERO;
                    let stop_all_clicked = footer_ui
                        .add_sized(
                            [SIDEBAR_WIDTH, CONTROLLER_ROW_HEIGHT],
                            egui::Button::new("Stop all controllers")
                                .fill(Color32::from_rgb(150, 45, 65)),
                        )
                        .clicked();
                    stop_all = stop_all_after_click(stop_all_clicked);
                    footer_ui.add_sized([SIDEBAR_WIDTH, 1.0], egui::Separator::default());

                    let (log_rect, _) = footer_ui.allocate_exact_size(
                        Vec2::new(SIDEBAR_WIDTH, log_height),
                        Sense::hover(),
                    );
                    footer_ui
                        .painter()
                        .rect_filled(log_rect, 0.0, Color32::from_gray(8));
                    let log_content_rect = egui::Rect::from_min_max(
                        log_rect.left_top()
                            + egui::vec2(4.0, f32::from(DIAGNOSTIC_LOG_TOP_MARGIN)),
                        log_rect.right_bottom() - egui::vec2(4.0, 0.0),
                    );
                    let mut log_ui = footer_ui.new_child(
                        egui::UiBuilder::new().max_rect(log_content_rect),
                    );
                    let log_scroll_height = diagnostic_log_scroll_height(log_height);
                    egui::ScrollArea::vertical()
                        .id_salt("diagnostic_log")
                        .auto_shrink([false, false])
                        .min_scrolled_height(log_scroll_height)
                        .max_height(log_scroll_height)
                        .stick_to_bottom(true)
                        .show(&mut log_ui, |ui| {
                            ui.set_width(SIDEBAR_WIDTH - 16.0);
                            ui.add_space(diagnostic_log_top_padding(
                                log_scroll_height,
                                ui.text_style_height(&egui::TextStyle::Body),
                                self.diagnostic_log.len(),
                                ui.spacing().item_spacing.y,
                            ));
                            if self.diagnostic_log.is_empty() {
                                ui.weak("Warnings and error codes will appear here");
                            }
                            for entry in &self.diagnostic_log {
                                ui.colored_label(
                                    if entry.success {
                                        Color32::from_rgb(105, 170, 105)
                                    } else {
                                        Color32::RED
                                    },
                                    &entry.message,
                                );
                            }
                        });

                    let status_color = if self.backend_healthy {
                        Color32::GREEN
                    } else {
                        Color32::RED
                    };
                    footer_ui.allocate_ui_with_layout(
                        Vec2::new(SIDEBAR_WIDTH, CONTROLLER_ROW_HEIGHT),
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            let dump_width = 76.0;
                            dump_state |= ui
                                .add_sized(
                                    [dump_width, CONTROLLER_ROW_HEIGHT],
                                    Button::new("Dump log"),
                                )
                                .clicked();
                            ui.with_layout(
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.colored_label(
                                        status_color,
                                        format!(
                                            "● {}",
                                            if self.backend_healthy {
                                                "Healthy"
                                            } else {
                                                "Attention"
                                            }
                                        ),
                                    );
                                },
                            );
                        },
                    );
                    footer_ui.allocate_space(Vec2::new(SIDEBAR_WIDTH, HEALTH_BOTTOM_PADDING));
                });
                ui.separator();
                ui.vertical(|ui| {
                    ui.set_min_width(448.0);
                    egui::ScrollArea::vertical()
                        .id_salt("live_panel_scroll")
                        .max_height(ui.available_height())
                        .auto_shrink([false, false])
                        .scroll_source(egui::scroll_area::ScrollSource {
                            scroll_bar: true,
                            drag: false,
                            mouse_wheel: true,
                        })
                        .show(ui, |ui| {
                    ui.set_min_width(448.0);
                    let mut polling_period_seconds = self.polling_period_seconds;
                    if let Some(index) = self
                        .selected_controller
                        .filter(|index| *index < self.controllers.len())
                    {
                        let named = &mut self.controllers[index];
                        ui.heading(&named.name);
                        ui.add_sized([ui.available_width(), 1.0], egui::Separator::default());
                        draw_controller_state(ui, named, &mut polling_period_seconds);
                        let input_width = ui.available_width();
                        ui.group(|ui| {
                            ui.set_min_width(input_width - 8.0);
                            let inputs_ready = named.edits.ready();
                            draw_battery_emulation(ui, &mut named.view, inputs_ready);
                            if inputs_ready {
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
                        });
                        draw_reverse_output_log(ui, &mut named.output_log);
                    }
                    self.polling_period_seconds = polling_period_seconds;
                        });
                });
                });
            });
        });
        if dump_state {
            self.write_state_dump();
        }
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

fn draw_controller_state(
    ui: &mut egui::Ui,
    controller: &mut NamedController,
    polling_period_seconds: &mut u32,
) {
    let identifier = controller_identifier(controller);
    let target = target_label(controller.options.target);
    ui.group(|ui| {
        egui::Grid::new("controller_topology")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                ui.label("Name");
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [180.0, NAME_INPUT_HEIGHT],
                        egui::TextEdit::singleline(&mut controller.name_draft)
                            .vertical_align(egui::Align::Center),
                    );
                    if ui.button("Apply").clicked() {
                        apply_controller_name(controller);
                    }
                });
                ui.end_row();
                if let Some(error) = &controller.name_error {
                    ui.label("");
                    ui.colored_label(Color32::RED, error);
                    ui.end_row();
                }
                ui.label("ID");
                ui.monospace(identifier);
                ui.end_row();
                ui.label("Target");
                ui.label(target);
                ui.end_row();
                ui.label("Type");
                ui.label(controller.kind.label());
                ui.end_row();
            });
        ui.separator();
        if let Some(worker) = &controller.service_worker {
            if let Ok(display) = worker.display.try_lock() {
                draw_state_row(ui, "Service cycles", |ui| {
                    ui.label(display.metrics.cycles.to_string());
                });
                draw_state_row(ui, "Omitted logs", |ui| {
                    ui.label(display.metrics.omitted_logs.to_string());
                });
                draw_state_row(ui, "Max gap", |ui| {
                    let gaps = service_gap_percentiles(&display.metrics, *polling_period_seconds);
                    for (label, gap) in [
                        ("10%:", gaps.map(|gaps| gaps[0])),
                        ("1%:", gaps.map(|gaps| gaps[1])),
                        ("0.1%:", gaps.map(|gaps| gaps[2])),
                    ] {
                        ui.label(label);
                        if let Some(gap) = gap {
                            ui.monospace(format_gap(gap));
                        } else {
                            ui.weak("—");
                        }
                    }
                    ui.label("Polling period:");
                    ui.add_sized(
                        [38.0, NAME_INPUT_HEIGHT],
                        egui::DragValue::new(polling_period_seconds)
                            .speed(1.0)
                            .suffix("s"),
                    )
                    .on_hover_text("0 includes the controller's entire observed lifetime");
                });
            }
        }
        ui.separator();
        draw_led_line(ui, &controller.indicators);
        draw_rumble_line(ui, &controller.indicators);
    });
}

fn format_gap(gap: Duration) -> String {
    format!("{:.2} ms", gap.as_secs_f64() * 1000.0)
}

fn draw_state_row_label(ui: &mut egui::Ui, label: &str) {
    ui.add_sized(
        [STATE_ROW_LABEL_WIDTH, NAME_INPUT_HEIGHT],
        egui::Label::new(label),
    );
}

fn draw_state_row(ui: &mut egui::Ui, label: &str, contents: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        draw_state_row_label(ui, label);
        contents(ui);
    });
}

fn draw_led_line(ui: &mut egui::Ui, indicators: &ReverseIndicators) {
    draw_state_row(ui, "LED", |ui| {
        let lightbar = indicators.led.unwrap_or([30, 30, 30]);
        draw_feedback_indicator(
            ui,
            "Lightbar",
            Color32::from_rgb(lightbar[0], lightbar[1], lightbar[2]),
            if indicators.led.is_some() {
                "Lightbar output received"
            } else {
                "Lightbar output unknown"
            },
        );
        draw_feedback_indicator(
            ui,
            "Mute",
            if indicators.mute_led == Some(true) {
                Color32::from_rgb(255, 130, 40)
            } else {
                Color32::DARK_GRAY
            },
            match indicators.mute_led {
                Some(true) => "Mute LED on",
                Some(false) => "Mute LED off",
                None => "Mute LED unknown",
            },
        );
    });
}

fn draw_rumble_line(ui: &mut egui::Ui, indicators: &ReverseIndicators) {
    let remaining = indicators
        .rumble_until
        .map(|until| until.saturating_duration_since(Instant::now()))
        .unwrap_or_default();
    let active = indicators.rumble_active || !remaining.is_zero();
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
    draw_state_row(ui, "Rumble", |ui| {
        draw_feedback_indicator(
            ui,
            "Motors",
            if active {
                Color32::from_rgb(220, 80, 80)
            } else {
                Color32::DARK_GRAY
            },
            if !indicators.rumble_seen {
                "Rumble unknown"
            } else if active {
                "Rumble active"
            } else {
                "Rumble inactive"
            },
        );
        if active {
            let (pulse_rect, _) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
            ui.painter().circle_stroke(
                pulse_rect.center(),
                radius,
                Stroke::new(1.0, Color32::from_rgb(220, 80, 80)),
            );
        }
    });
}

fn draw_feedback_indicator(ui: &mut egui::Ui, label: &str, color: Color32, tooltip: &str) {
    ui.label(format!("{label}:"));
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 5.0, color);
    response.on_hover_text(tooltip);
}

fn draw_battery_emulation(ui: &mut egui::Ui, view: &mut ControllerView, editable: bool) {
    let supported = view.supports_battery_emulation();
    let battery = view.battery();
    let mut exposed = battery.is_exposed();
    ui.horizontal(|ui| {
        draw_state_row_label(ui, "Battery");
        let expose_response = ui.add_enabled(
            supported && editable,
            egui::Checkbox::new(&mut exposed, "Expose"),
        );
        if expose_response.changed() {
            let _ = view.set_battery_exposed(exposed);
        }
        if battery_controls_are_active(supported && editable, exposed) {
            let mut percentage = battery.level().percent();
            let slider_changed = ui
                .add_sized(
                    [120.0, NAME_INPUT_HEIGHT],
                    egui::Slider::new(&mut percentage, 0..=100).show_value(false),
                )
                .changed();
            let entry_changed = ui
                .add_sized(
                    [56.0, NAME_INPUT_HEIGHT],
                    egui::DragValue::new(&mut percentage)
                        .range(0..=100)
                        .suffix("%"),
                )
                .changed();
            if (slider_changed || entry_changed)
                && let Ok(level) = BatteryLevel::new(percentage)
            {
                let _ = view.set_battery_level(level);
            }
        } else {
            draw_inactive_battery_value(ui, 120.0);
            draw_inactive_battery_value(ui, 56.0);
            if !supported {
                ui.weak("unsupported");
            }
        }
    });
}

const fn battery_controls_are_active(supported: bool, exposed: bool) -> bool {
    supported && exposed
}

fn draw_inactive_battery_value(ui: &mut egui::Ui, width: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, NAME_INPUT_HEIGHT), Sense::hover());
    ui.painter().rect_filled(rect, 2.0, Color32::from_gray(24));
    ui.painter().rect_stroke(
        rect,
        2.0,
        Stroke::new(1.0, Color32::from_gray(42)),
        egui::StrokeKind::Inside,
    );
}

fn draw_reverse_output_log(ui: &mut egui::Ui, output_log: &mut Vec<String>) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label("Reverse output log");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button("Clear")
                    .on_hover_text("Clear reverse output")
                    .clicked()
                {
                    output_log.clear();
                }
            });
        });
        egui::Frame::NONE
            .fill(Color32::from_gray(8))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("selected_controller_reverse_output")
                    .max_height(NAME_INPUT_HEIGHT * 5.0)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        if output_log.is_empty() {
                            ui.weak("No reverse output received.");
                        } else {
                            for entry in output_log.iter() {
                                ui.monospace(entry);
                            }
                        }
                    });
            });
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
    fn controller_ids_use_the_requested_target_and_type_abbreviations() {
        assert_eq!(
            controller_id(0, RealizationId::LINUX_UINPUT, Kind::Xbox360),
            "000-UIN-XB360"
        );
        assert_eq!(
            controller_id(7, RealizationId::LINUX_UHID_USB, Kind::DualSense),
            "007-HID-DUALSENSE"
        );
        assert_eq!(
            controller_id(
                1_234,
                RealizationId::LINUX_DUMMY_HCD_USB_HID,
                Kind::SwitchPro
            ),
            "234-USB-SWITCHPRO"
        );
    }

    #[test]
    fn target_help_is_available_only_for_the_experimental_gadget_target() {
        assert!(target_help(RealizationId::LINUX_UINPUT).is_none());
        assert!(target_help(RealizationId::LINUX_UHID_USB).is_none());
        let help = target_help(RealizationId::LINUX_DUMMY_HCD_USB_HID)
            .expect("dummy_hcd has experimental-target help");
        assert_eq!(help.title, "Experimental USB gadget");
    }

    #[test]
    fn state_dump_reports_program_state_without_a_live_controller() {
        let dump = App::default().state_dump();
        assert!(dump.starts_with("virtualgamepad demo state dump"));
        assert!(dump.contains("Backend health: healthy"));
        assert!(dump.contains("Controller count: 0"));
        assert!(dump.contains("GUI diagnostic log:"));
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
                Duration::from_millis(16)
            );
            assert_eq!(
                service_repaint_interval(count, None),
                Duration::from_millis(16)
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
        assert_eq!(repaint_interval(1), Duration::from_millis(16));
        assert_eq!(repaint_interval(8), Duration::from_millis(16));
    }

    #[test]
    fn sidebar_budget_reserves_space_for_list_and_footer_without_overlap() {
        let layout = sidebar_layout_budget(500.0, 50.0, 74.0, 3.0);
        assert!((layout.diagnostic_log - 74.0).abs() < 0.001);
        assert!((layout.footer - 124.0).abs() < 0.001);
        assert!((layout.controller_list - 362.0).abs() < 0.001);
        assert!(
            (layout.controller_list + CONTROLLER_LIST_FRAME_VERTICAL_MARGIN + layout.footer + 6.0
                - 500.0)
                .abs()
                < 0.001
        );

        let constrained = sidebar_layout_budget(90.0, 50.0, 74.0, 3.0);
        assert!((constrained.diagnostic_log - 74.0).abs() < 0.001);
        assert!((constrained.controller_list - CONTROLLER_LIST_MIN_HEIGHT).abs() < 0.001);
    }

    #[test]
    fn diagnostic_log_has_room_for_five_lines() {
        assert!((diagnostic_log_height(14.0) - 74.0).abs() < 0.001);
        assert!((diagnostic_log_scroll_height(74.0) - 70.0).abs() < 0.001);
        assert!(diagnostic_log_scroll_height(2.0).abs() < 0.001);
    }

    #[test]
    fn diagnostic_log_bottom_aligns_without_expanding_short_content() {
        assert!((diagnostic_log_top_padding(70.0, 14.0, 4, 3.0) - 5.0).abs() < 0.001);
        assert!(diagnostic_log_top_padding(70.0, 14.0, 5, 3.0).abs() < 0.001);
        assert!(diagnostic_log_top_padding(70.0, 14.0, 9, 3.0).abs() < 0.001);
    }

    #[test]
    fn advanced_options_are_available_for_every_target() {
        for target in [
            RealizationId::LINUX_UINPUT,
            RealizationId::LINUX_UHID_USB,
            RealizationId::LINUX_DUMMY_HCD_USB_HID,
        ] {
            assert!(advanced_options_available(target));
        }
    }

    #[test]
    fn advanced_disclosure_arrow_tracks_expansion() {
        assert_eq!(
            advanced_disclosure_direction(false),
            DisclosureDirection::Right
        );
        assert_eq!(
            advanced_disclosure_direction(true),
            DisclosureDirection::Down
        );
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
    fn reverse_output_logs_identify_the_controller_context() {
        let mut logs = vec!["ForceFeedback".to_owned(), "HidOutput".to_owned()];
        label_output_logs("DualSense 1 · 007-HID-DUALSENSE", &mut logs);
        assert_eq!(
            logs,
            vec![
                "DualSense 1 · 007-HID-DUALSENSE: ForceFeedback",
                "DualSense 1 · 007-HID-DUALSENSE: HidOutput"
            ]
        );
    }

    #[test]
    fn battery_controls_require_an_exposed_supported_battery() {
        assert!(!battery_controls_are_active(false, false));
        assert!(!battery_controls_are_active(false, true));
        assert!(!battery_controls_are_active(true, false));
        assert!(battery_controls_are_active(true, true));
    }

    #[test]
    fn service_gap_tail_percentiles_cover_the_requested_observation_window() {
        let start = Instant::now();
        let mut metrics = ServiceMetrics::default();
        metrics.record(start);
        let mut elapsed = 0;
        for milliseconds in 1..=10 {
            elapsed += milliseconds;
            metrics.record(start + Duration::from_millis(elapsed));
        }
        assert_eq!(
            service_gap_percentiles(&metrics, 0),
            Some([
                Duration::from_millis(9),
                Duration::from_millis(10),
                Duration::from_millis(10)
            ])
        );
        assert_eq!(
            service_gap_percentiles(&metrics, 1),
            Some([
                Duration::from_millis(9),
                Duration::from_millis(10),
                Duration::from_millis(10)
            ])
        );
        assert_eq!(format_gap(Duration::from_micros(1_250)), "1.25 ms");
    }

    #[test]
    fn removing_any_tab_selects_the_nearest_remaining_controller() {
        assert_eq!(selection_after_removal(0, 0, Some(0)), None);
        assert_eq!(selection_after_removal(2, 0, Some(0)), Some(0));
        assert_eq!(selection_after_removal(2, 1, Some(1)), Some(1));
        assert_eq!(selection_after_removal(2, 2, Some(2)), Some(1));
    }

    #[test]
    fn removing_another_tab_preserves_the_selected_controller() {
        assert_eq!(selection_after_removal(2, 0, Some(2)), Some(1));
        assert_eq!(selection_after_removal(2, 2, Some(0)), Some(0));
        assert_eq!(selection_after_removal(2, 1, None), None);
    }

    #[test]
    fn automatic_controller_names_reuse_only_unused_suffixes() {
        assert_eq!(
            next_available_name(
                Kind::DualSense,
                ["DualSense 0".to_owned(), "DualSense 2".to_owned()].into_iter(),
            ),
            "DualSense 1"
        );
        assert_eq!(
            next_available_name(Kind::Xbox360, ["DualSense 0".to_owned()].into_iter(),),
            "Xbox 360 0"
        );
    }

    #[test]
    fn controller_name_sanitization_trims_and_rejects_invalid_drafts() {
        assert_eq!(
            sanitized_controller_name("  Player one  "),
            Ok("Player one".into())
        );
        assert_eq!(
            sanitized_controller_name("\t\n"),
            Err("Name cannot be empty")
        );
        assert_eq!(
            sanitized_controller_name("name\nnext"),
            Err("Name cannot contain control characters")
        );
        assert_eq!(
            sanitized_controller_name(&"x".repeat(CONTROLLER_NAME_MAX_CHARS + 1)),
            Err("Name must be 64 characters or fewer")
        );
    }

    #[test]
    fn default_creation_count_is_bounded_to_automatic_names() {
        assert_eq!(requested_create_count("", 0), 1);
        assert_eq!(requested_create_count("  ", 4), 4);
        assert_eq!(requested_create_count("Named pad", 9), 1);
    }

    #[test]
    fn create_count_spinbox_steps_without_crossing_its_minimum() {
        assert_eq!(step_create_count(1, false), 1);
        assert_eq!(step_create_count(4, false), 3);
        assert_eq!(step_create_count(9_999, true), 10_000);
    }

    #[test]
    fn controller_rows_match_the_name_input_height() {
        assert!((CONTROLLER_ROW_HEIGHT - NAME_INPUT_HEIGHT).abs() < f32::EPSILON);
        assert!((CONTROLLER_DELETE_WIDTH - NAME_INPUT_HEIGHT).abs() < f32::EPSILON);
    }

    #[test]
    fn spinbox_arrows_stay_inside_the_number_field() {
        let field = egui::Rect::from_min_size(Pos2::new(20.0, 30.0), Vec2::new(58.0, 22.0));
        let (increment, decrement) = spinbox_arrow_rects(field, 16.0);

        assert!(field.contains_rect(increment));
        assert!(field.contains_rect(decrement));
        assert!((increment.bottom() - decrement.top()).abs() < f32::EPSILON);
        assert!((increment.right() - field.right()).abs() < f32::EPSILON);
        assert!((decrement.right() - field.right()).abs() < f32::EPSILON);
    }

    #[test]
    fn controller_identifier_is_truncated_for_sidebar_rows() {
        assert_eq!(truncate_identifier("abc", 4), "abc");
        assert_eq!(truncate_identifier("abcdef", 4), "abcd…");
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
    fn controller_row_click_selects_the_clicked_controller() {
        assert_eq!(selection_after_controller_click(Some(0), 3, true), Some(3));
        assert_eq!(selection_after_controller_click(Some(3), 1, false), Some(3));
    }

    #[test]
    fn controller_delete_click_requests_immediate_removal() {
        assert_eq!(controller_removal_after_delete_click(3, false), None);
        assert_eq!(controller_removal_after_delete_click(3, true), Some(3));
    }

    #[test]
    fn successful_controller_close_is_logged() {
        assert_eq!(
            successful_controller_close_message("Xbox 360 0"),
            "Closed Xbox 360 0."
        );
    }

    #[test]
    fn stop_all_click_requests_immediate_cleanup() {
        assert!(!stop_all_after_click(false));
        assert!(stop_all_after_click(true));
    }

    #[test]
    fn controller_rows_use_a_hover_outline_without_overriding_selection() {
        let selected = Color32::BLUE;

        assert_eq!(controller_row_fill(false, selected), Color32::from_gray(62));
        assert_eq!(controller_row_fill(true, selected), selected);
        assert!(controller_row_has_hover_outline(true));
        assert!(!controller_row_has_hover_outline(false));
        assert_eq!(
            sidebar_item_text_color(true, Color32::GRAY, Color32::WHITE),
            Color32::WHITE
        );
        assert_eq!(
            sidebar_item_text_color(false, Color32::GRAY, Color32::WHITE),
            Color32::GRAY
        );
        assert_eq!(
            controller_row_text_color(true, false, Color32::GRAY, Color32::WHITE),
            SELECTED_CONTROLLER_TEXT
        );
        assert_eq!(
            controller_row_text_color(false, true, Color32::GRAY, Color32::WHITE),
            Color32::WHITE
        );
    }

    #[test]
    fn only_an_open_backend_reports_healthy() {
        assert!(backend_status_is_healthy(ControllerStatus::Open));
        assert!(!backend_status_is_healthy(ControllerStatus::NotOpen));
        assert!(!backend_status_is_healthy(ControllerStatus::Closed));
        assert!(!backend_status_is_healthy(ControllerStatus::Failed));
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
    fn reverse_indicators_start_unknown_until_host_output_is_observed() {
        let mut indicators = ReverseIndicators::default();
        assert!(indicators.led.is_none());
        assert!(indicators.mute_led.is_none());
        assert!(!indicators.rumble_seen);

        indicators.apply_hid_output(None, None, None, Some(false));
        assert_eq!(indicators.mute_led, Some(false));
        assert!(!indicators.rumble_seen);

        indicators.apply_hid_output(Some(0), Some(0), None, None);
        assert!(indicators.rumble_seen);
        assert!(!indicators.rumble_active);
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
