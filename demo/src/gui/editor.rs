//! Detached native-state views and bounded edits for the demo only.
use super::{
    BatteryLevel, BatteryState, Controller, DigitalControlUpdate, DualSenseAxis, DualSenseControl,
    DualSenseTouchContact, DualSenseTrigger, DualShock4Axis, DualShock4Control,
    DualShock4MotionSample, DualShock4TouchContact, DualShock4TouchSlot, DualShock4Trigger,
    MotionSample, SwitchProAxis, SwitchProControl, SwitchProMotionSample, TouchSlot, Xbox360Axis,
    Xbox360Control, Xbox360Trigger,
};
use virtualgamepad::{
    ControllerDiagnostics, DualSenseInputs, DualShock4Inputs, SwitchProInputs, Xbox360State,
};

pub(super) const EDIT_LIMIT: usize = 64;
pub(super) type Command<C> = Box<dyn FnOnce(&mut C) -> Result<(), String> + Send>;
type Edit = Command<Controller>;
pub(super) type EditBatch = Vec<Edit>;

pub(super) struct Editor<S, T: 'static, M = ()> {
    state: S,
    metadata: M,
    surface: T,
    edits: EditBatch,
    overflow: bool,
    diagnostics: Option<ControllerDiagnostics>,
    association: Option<virtualgamepad::ControllerAssociation>,
    feedback: std::sync::Arc<std::sync::Mutex<EditorFeedback>>,
}
#[derive(Default)]
struct EditorFeedback {
    error: Option<String>,
    diagnostics: Vec<super::DiagnosticLogEntry>,
}

fn record_wii_result(
    feedback: &std::sync::Mutex<EditorFeedback>,
    operation: &str,
    result: &Result<(), String>,
) {
    let mut feedback = feedback.lock().expect("editor feedback lock");
    feedback.error = result
        .as_ref()
        .err()
        .map(|error| format!("{operation}: {error}"));
    if let Some(message) = feedback.error.clone() {
        if feedback.diagnostics.len() == EDIT_LIMIT {
            feedback.diagnostics.remove(0);
        }
        feedback.diagnostics.push(super::DiagnosticLogEntry {
            message,
            success: false,
        });
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WiiExtensionChoice {
    None,
    Nunchuk,
    Classic,
    ClassicPro,
    Unknown,
}
impl WiiExtensionChoice {
    pub(super) const ALL: [Self; 4] = [Self::None, Self::Nunchuk, Self::Classic, Self::ClassicPro];
    pub(super) fn from_extension(value: &gr_curated_controllers::wii_remote::WiiExtension) -> Self {
        use gr_curated_controllers::wii_remote::WiiExtension;
        match value {
            WiiExtension::None => Self::None,
            WiiExtension::Nunchuk(_) => Self::Nunchuk,
            WiiExtension::Classic(_) => Self::Classic,
            WiiExtension::ClassicPro(_) => Self::ClassicPro,
            _ => Self::Unknown,
        }
    }
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Nunchuk => "Nunchuk",
            Self::Classic => "Classic",
            Self::ClassicPro => "Classic Pro",
            Self::Unknown => "Unknown extension",
        }
    }
    fn extension(self) -> Result<gr_curated_controllers::wii_remote::WiiExtension, String> {
        use gr_curated_controllers::wii_remote::{WiiClassicState, WiiExtension, WiiNunchukState};
        Ok(match self {
            Self::None => WiiExtension::None,
            Self::Nunchuk => WiiExtension::Nunchuk(WiiNunchukState::default()),
            Self::Classic => WiiExtension::Classic(WiiClassicState::default()),
            Self::ClassicPro => WiiExtension::ClassicPro(WiiClassicState::default()),
            Self::Unknown => return Err("extension requires a declared native editor".into()),
        })
    }
}

impl<S, T, M> Editor<S, T, M> {
    pub(super) fn state(&self) -> &S {
        &self.state
    }
    pub(super) fn surface(&self) -> &T {
        &self.surface
    }
    pub(super) fn queue(&mut self, edit: Edit) -> Result<(), String> {
        if self.edits.len() == EDIT_LIMIT {
            self.overflow = true;
            return Err("native edit batch exceeded its bound".into());
        }
        self.edits.push(edit);
        Ok(())
    }
    pub(super) fn take_edits(&mut self) -> Result<EditBatch, String> {
        if self.overflow {
            return Err("native edit batch exceeded its bound".into());
        }
        Ok(std::mem::take(&mut self.edits))
    }
}
pub(super) struct Surface(pub(super) &'static virtualgamepad::ControllerSurface);
impl Surface {
    pub(super) const fn common(&self) -> &'static virtualgamepad::ControllerSurface {
        self.0
    }
}
impl virtualgamepad::ControllerSurfaceInfo for Surface {
    fn common_surface(&self) -> &virtualgamepad::ControllerSurface {
        self.0
    }
}
pub(super) type XboxSeriesEditor = Editor<virtualgamepad::XboxSeriesState, Surface>;
pub(super) type Xbox360Editor = Editor<Xbox360State, Surface>;
pub(super) type DualSenseEditor = Editor<DualSenseInputs, Surface, virtualgamepad::DualSenseStatus>;
pub(super) type DualShock4Editor =
    Editor<DualShock4Inputs, Surface, virtualgamepad::DualShock4Status>;
pub(super) type SwitchProEditor = Editor<SwitchProInputs, Surface, virtualgamepad::SwitchProStatus>;
pub(super) type WiiRemoteEditor =
    Editor<gr_curated_controllers::wii_remote::WiiRemoteState, Surface>;
pub(super) type SteamEditor =
    Editor<gr_curated_controllers::steam_controller_2026::SteamController2026State, Surface>;
pub(super) enum ControllerView {
    Xbox(Xbox360Editor),
    XboxSeries(XboxSeriesEditor),
    DualSense(DualSenseEditor),
    DualShock4(DualShock4Editor),
    SwitchPro(SwitchProEditor),
    WiiRemote(WiiRemoteEditor),
    Steam(SteamEditor),
}
impl<S, M> Editor<S, Surface, M> {
    fn new(
        state: S,
        surface: &'static virtualgamepad::ControllerSurface,
        diagnostics: Option<ControllerDiagnostics>,
        association: Option<virtualgamepad::ControllerAssociation>,
        metadata: M,
    ) -> Self {
        Self {
            state,
            metadata,
            surface: Surface(surface),
            edits: Vec::new(),
            overflow: false,
            diagnostics,
            association,
            feedback: std::sync::Arc::default(),
        }
    }
}
impl Controller {
    pub(super) fn snapshot(&mut self) -> ControllerView {
        macro_rules! view {
            ($c:ident, $variant:ident) => {
                ControllerView::$variant(Editor::new(
                    $c.state().clone(),
                    $c.surface().common(),
                    Some($c.diagnostics()),
                    Some($c.association().clone()),
                    (),
                ))
            };
        }
        match self {
            Self::Xbox(c) => view!(c, Xbox),
            Self::XboxSeries(c) => ControllerView::XboxSeries(Editor::new(
                c.state().clone(),
                c.surface(),
                Some(c.diagnostics()),
                Some(c.association().clone()),
                (),
            )),
            Self::DualSense(c) => ControllerView::DualSense(Editor::new(
                c.inputs().clone(),
                c.surface().common(),
                Some(c.diagnostics()),
                Some(c.association().clone()),
                c.status(),
            )),
            Self::DualShock4(c) => ControllerView::DualShock4(Editor::new(
                c.inputs().clone(),
                c.surface().common(),
                Some(c.diagnostics()),
                Some(c.association().clone()),
                c.status(),
            )),
            Self::SwitchPro(c) => ControllerView::SwitchPro(Editor::new(
                c.inputs().clone(),
                c.surface().common(),
                Some(c.diagnostics()),
                Some(c.association().clone()),
                c.status(),
            )),
            Self::Steam(c) => {
                ControllerView::Steam(Editor::new(c.state().clone(), c.surface(), None, None, ()))
            }
            Self::WiiRemote(c) => ControllerView::WiiRemote(Editor::new(
                c.state().clone(),
                c.surface(),
                None,
                None,
                (),
            )),
        }
    }
}
fn prepare_wii_input(
    current: &gr_curated_controllers::wii_remote::WiiRemoteState,
    event: &super::InputEvent,
) -> Result<gr_curated_controllers::wii_remote::WiiRemoteState, String> {
    let mut state = current.clone();
    if super::native_editor::apply_wii_event(&mut state, event) {
        Ok(state)
    } else {
        Err("input is unavailable in the retained Wii topology or outside its native range".into())
    }
}

impl WiiRemoteEditor {
    fn queue_wii_edit(&mut self, edit: Edit) -> Result<(), String> {
        // A refused topology/IR edit leaves the existing batch serviceable.
        if self.edits.len() == EDIT_LIMIT {
            return Err("native edit batch exceeded its bound".into());
        }
        self.queue(edit)
    }
    pub(super) fn edit_wii(&mut self, event: &super::InputEvent) {
        if !super::native_editor::apply_wii_event(&mut self.state, event) {
            return;
        }
        let event = event.clone();
        let feedback = self.feedback.clone();
        let _ = self.queue(Box::new(move |c| {
            let Controller::WiiRemote(c) = c else {
                return Err("Wii editor mismatch".into());
            };
            // A preceding attach can fail after GUI preflight. Apply the event to
            // the worker's retained topology, never an optimistic whole-state copy.
            match prepare_wii_input(c.state(), &event) {
                Ok(state) => c.set_inputs(state).map_err(|e| e.to_string()),
                Err(error) => {
                    record_wii_result(&feedback, "Input", &Err(error));
                    Ok(())
                }
            }
        }));
    }
    fn attach_wii(
        &mut self,
        value: gr_curated_controllers::wii_remote::WiiExtension,
        mp: bool,
    ) -> Result<(), String> {
        let mut candidate = self.state.clone();
        candidate
            .set_attachment(value.clone(), mp)
            .map_err(|e| e.to_string())?;
        let feedback = self.feedback.clone();
        self.queue_wii_edit(Box::new(move |c| {
            let Controller::WiiRemote(c) = c else {
                return Err("Wii editor mismatch".into());
            };
            let result = c.attach(value, mp).map_err(|error| error.to_string());
            record_wii_result(&feedback, "Attachment", &result);
            // Rejection is recoverable: native attachment owns rollback/neutralization.
            Ok(())
        }))?;
        self.state = candidate;
        Ok(())
    }
    pub(super) fn select_wii_attachment(&mut self, choice: WiiExtensionChoice, bridge: bool) {
        let result = choice
            .extension()
            .and_then(|extension| self.attach_wii(extension, bridge));
        record_wii_result(&self.feedback, "Attachment", &result);
    }
    pub(super) fn edit_wii_contact(&mut self, contact: u8, point: Option<(u32, u32)>) {
        use gr_curated_controllers::wii_remote::WiiIrPoint;
        let result = (|| {
            if usize::from(contact) >= self.state.ir().len() {
                return Err("IR contact outside the four-point set".to_owned());
            }
            let point = point
                .map(|(x, y)| {
                    let (Ok(x), Ok(y)) = (u16::try_from(x), u16::try_from(y)) else {
                        return Err(
                            "IR point outside the source-backed coordinate domain".to_owned()
                        );
                    };
                    WiiIrPoint::new(x, y).map_err(|error| error.to_string())
                })
                .transpose()?;
            let mut points = *self.state.ir();
            points[usize::from(contact)] = point;
            Ok(points)
        })();
        match result {
            Ok(points) => self.edit_wii_ir(points),
            Err(error) => record_wii_result(&self.feedback, "IR", &Err(error)),
        }
    }
    pub(super) fn edit_wii_ir(
        &mut self,
        points: [Option<gr_curated_controllers::wii_remote::WiiIrPoint>; 4],
    ) {
        let feedback = self.feedback.clone();
        let result = self.queue_wii_edit(Box::new(move |c| {
            let Controller::WiiRemote(c) = c else {
                return Err("Wii editor mismatch".into());
            };
            let result = c.set_ir(points).map_err(|error| error.to_string());
            record_wii_result(&feedback, "IR", &result);
            Ok(())
        }));
        if result.is_ok() {
            self.state.set_ir(points);
        }
        record_wii_result(&self.feedback, "IR", &result);
    }
    pub(super) fn wii_topology_name(&self) -> String {
        let extension = WiiExtensionChoice::from_extension(self.state.extension());
        let mut name = "Wii Remote".to_owned();
        if self.state.motion_plus().attached {
            name.push_str(" + MotionPlus");
        }
        if extension != WiiExtensionChoice::None {
            name.push_str(" + ");
            name.push_str(extension.label());
        }
        name
    }
    pub(super) fn draw_wii(
        &mut self,
        ui: &mut super::egui::Ui,
        controller_id: u64,
    ) -> super::egui::Rect {
        let row = super::input_clusters::horizontal_cards(
            ui,
            (controller_id, "attachment"),
            false,
            |ui| {
                super::input_clusters::card(ui, "Attachment", |ui| {
                    let mut extension = WiiExtensionChoice::from_extension(self.state.extension());
                    let before = extension;
                    let mut mp = self.state.motion_plus().attached;
                    let old_mp = mp;
                    ui.label("Extension");
                    ui.horizontal_wrapped(|ui| {
                        for choice in WiiExtensionChoice::ALL {
                            ui.selectable_value(&mut extension, choice, choice.label());
                        }
                    });
                    ui.checkbox(&mut mp, "MotionPlus bridge");
                    if extension != before || mp != old_mp {
                        self.select_wii_attachment(extension, mp);
                    }
                    ui.label(format!("Topology: {}", self.wii_topology_name()));
                    if extension == WiiExtensionChoice::ClassicPro {
                        ui.label("Classic Pro identity: No evidence");
                    }
                    if let Some(error) = &self.feedback.lock().expect("editor feedback lock").error
                    {
                        ui.colored_label(super::Color32::RED, error);
                    }
                });
            },
        );
        row.inner_rect
    }
}

#[cfg(test)]
pub(super) fn neutral_view(kind: super::Kind) -> ControllerView {
    let surface = (kind.entry().realizations)()
        .iter()
        .find(|r| r.surface.target() == virtualgamepad::RealizationId::LINUX_UINPUT)
        .expect("compiled uinput surface")
        .surface;
    match kind {
        super::Kind::XboxSeries => ControllerView::XboxSeries(Editor::new(
            virtualgamepad::XboxSeriesState::default(),
            surface,
            None,
            None,
            (),
        )),
        super::Kind::Xbox360 => ControllerView::Xbox(Editor::new(
            Xbox360State::default(),
            surface,
            None,
            None,
            (),
        )),
        super::Kind::DualSense => ControllerView::DualSense(Editor::new(
            DualSenseInputs::default(),
            surface,
            None,
            None,
            virtualgamepad::DualSenseStatus::default(),
        )),
        super::Kind::DualShock4 => ControllerView::DualShock4(Editor::new(
            DualShock4Inputs::default(),
            surface,
            None,
            None,
            virtualgamepad::DualShock4Status::default(),
        )),
        super::Kind::WiiRemote => ControllerView::WiiRemote(Editor::new(
            gr_curated_controllers::wii_remote::WiiRemoteState::default(),
            surface,
            None,
            None,
            (),
        )),
        super::Kind::Steam => ControllerView::Steam(Editor::new(
            gr_curated_controllers::steam_controller_2026::SteamController2026State::default(),
            surface,
            None,
            None,
            (),
        )),
        super::Kind::SwitchPro => ControllerView::SwitchPro(Editor::new(
            SwitchProInputs::default(),
            surface,
            None,
            None,
            virtualgamepad::SwitchProStatus::default(),
        )),
    }
}
#[cfg(test)]
pub(super) fn neutral_wii_view(revision: super::WiiRemoteRevision) -> ControllerView {
    let ControllerView::WiiRemote(mut editor) = neutral_view(super::Kind::WiiRemote) else {
        unreachable!()
    };
    editor.state = gr_curated_controllers::wii_remote::WiiRemoteState::new(revision);
    ControllerView::WiiRemote(editor)
}
impl ControllerView {
    pub(super) fn preserve_editor_feedback(&mut self, previous: &Self) {
        if let (Self::WiiRemote(next), Self::WiiRemote(previous)) = (self, previous) {
            next.feedback = previous.feedback.clone();
        }
    }
    pub(super) fn take_diagnostics(&mut self) -> Vec<super::DiagnosticLogEntry> {
        if let Self::WiiRemote(editor) = self {
            std::mem::take(
                &mut editor
                    .feedback
                    .lock()
                    .expect("editor feedback lock")
                    .diagnostics,
            )
        } else {
            Vec::new()
        }
    }
    pub(super) fn association(&self) -> Option<&virtualgamepad::ControllerAssociation> {
        match self {
            Self::Xbox(c) => c.association.as_ref(),
            Self::XboxSeries(c) => c.association.as_ref(),
            Self::DualSense(c) => c.association.as_ref(),
            Self::DualShock4(c) => c.association.as_ref(),
            Self::SwitchPro(c) => c.association.as_ref(),
            Self::WiiRemote(c) => c.association.as_ref(),
            Self::Steam(c) => c.association.as_ref(),
        }
    }
    pub(super) fn lab_details(&self) -> String {
        match self {
            Self::Xbox(c) => format!("{:?}; {:?}", c.association, c.diagnostics),
            Self::XboxSeries(c) => format!("{:?}; {:?}", c.association, c.diagnostics),
            Self::DualSense(c) => format!("{:?}; {:?}", c.association, c.diagnostics),
            Self::DualShock4(c) => format!("{:?}; {:?}", c.association, c.diagnostics),
            Self::SwitchPro(c) => format!("{:?}; {:?}", c.association, c.diagnostics),
            Self::Steam(_) => "Steam supporting package".into(),
            Self::WiiRemote(_) => "Wii supporting-package diagnostics and association are unavailable in this ordinary-root lab view.".into(),
        }
    }
    pub(super) fn release_inputs(&mut self) -> Result<(), String> {
        let edit = super::release_inputs();
        match self {
            Self::Xbox(c) => c.queue(edit),
            Self::XboxSeries(c) => c.queue(edit),
            Self::DualSense(c) => c.queue(edit),
            Self::DualShock4(c) => c.queue(edit),
            Self::SwitchPro(c) => c.queue(edit),
            Self::WiiRemote(c) => c.queue(edit),
            Self::Steam(c) => c.queue(edit),
        }
    }

    pub(super) fn take_edits(&mut self) -> Result<EditBatch, String> {
        match self {
            Self::Xbox(c) => c.take_edits(),
            Self::XboxSeries(c) => c.take_edits(),
            Self::DualSense(c) => c.take_edits(),
            Self::DualShock4(c) => c.take_edits(),
            Self::SwitchPro(c) => c.take_edits(),
            Self::WiiRemote(c) => c.take_edits(),
            Self::Steam(c) => c.take_edits(),
        }
    }
}
impl XboxSeriesEditor {
    pub(super) fn edit_xbox_series(&mut self, events: Vec<super::InputEvent>) {
        let mut next = self.state.clone();
        for event in events {
            if !super::native_editor::apply_xbox_series_event(&mut next, &event) {
                return;
            }
        }
        if next == self.state {
            return;
        }
        let queued = next.clone();
        if self
            .queue(Box::new(move |controller| {
                let Controller::XboxSeries(controller) = controller else {
                    return Err("edit family mismatch".into());
                };
                controller
                    .set_inputs(queued)
                    .map_err(|error| error.to_string())
            }))
            .is_ok()
        {
            self.state = next;
        }
    }
}

impl Xbox360Editor {
    pub(super) fn set_digital(&mut self, update: DigitalControlUpdate) -> Result<(), String> {
        self.queue(Box::new(move |controller| {
            let Controller::Xbox(controller) = controller else {
                return Err("edit family mismatch".into());
            };
            controller
                .set_digital(update)
                .map_err(|error| error.to_string())
        }))
    }
    pub(super) fn set_native(
        &mut self,
        control: Xbox360Control,
        pressed: bool,
    ) -> Result<(), String> {
        self.queue(Box::new(move |controller| {
            let Controller::Xbox(controller) = controller else {
                return Err("edit family mismatch".into());
            };
            controller
                .set_native(control, pressed)
                .map_err(|error| error.to_string())
        }))
    }
    pub(super) fn set_left_stick(&mut self, x: Xbox360Axis, y: Xbox360Axis) -> Result<(), String> {
        self.queue(Box::new(move |controller| {
            let Controller::Xbox(controller) = controller else {
                return Err("edit family mismatch".into());
            };
            controller
                .set_left_stick(x, y)
                .map_err(|error| error.to_string())
        }))
    }
    pub(super) fn set_right_stick(&mut self, x: Xbox360Axis, y: Xbox360Axis) -> Result<(), String> {
        self.queue(Box::new(move |controller| {
            let Controller::Xbox(controller) = controller else {
                return Err("edit family mismatch".into());
            };
            controller
                .set_right_stick(x, y)
                .map_err(|error| error.to_string())
        }))
    }
    pub(super) fn set_triggers(
        &mut self,
        left: Xbox360Trigger,
        right: Xbox360Trigger,
    ) -> Result<(), String> {
        self.queue(Box::new(move |controller| {
            let Controller::Xbox(controller) = controller else {
                return Err("edit family mismatch".into());
            };
            controller
                .set_triggers(left, right)
                .map_err(|error| error.to_string())
        }))
    }
    pub(super) fn set_battery_exposed(&mut self, exposed: bool) -> Result<(), String> {
        self.queue(Box::new(move |controller| {
            let Controller::Xbox(controller) = controller else {
                return Err("edit family mismatch".into());
            };
            controller
                .set_battery_exposed(exposed)
                .map_err(|error| error.to_string())
        }))
    }
    pub(super) fn set_battery_level(&mut self, level: BatteryLevel) -> Result<(), String> {
        self.queue(Box::new(move |controller| {
            let Controller::Xbox(controller) = controller else {
                return Err("edit family mismatch".into());
            };
            controller
                .set_battery_level(level)
                .map_err(|error| error.to_string())
        }))
    }
}
impl DualSenseEditor {
    pub(super) fn battery(&self) -> BatteryState {
        self.metadata.battery()
    }
    pub(super) fn set_digital(&mut self, update: DigitalControlUpdate) -> Result<(), String> {
        self.queue(Box::new(move |controller| {
            let Controller::DualSense(controller) = controller else {
                return Err("edit family mismatch".into());
            };
            controller
                .set_digital(update)
                .map_err(|error| error.to_string())
        }))
    }
    pub(super) fn set_native(
        &mut self,
        control: DualSenseControl,
        pressed: bool,
    ) -> Result<(), String> {
        self.queue(Box::new(move |controller| {
            let Controller::DualSense(controller) = controller else {
                return Err("edit family mismatch".into());
            };
            controller
                .set_native(control, pressed)
                .map_err(|error| error.to_string())
        }))
    }
    pub(super) fn set_left_stick(
        &mut self,
        x: DualSenseAxis,
        y: DualSenseAxis,
    ) -> Result<(), String> {
        self.queue(Box::new(move |controller| {
            let Controller::DualSense(controller) = controller else {
                return Err("edit family mismatch".into());
            };
            controller
                .set_left_stick(x, y)
                .map_err(|error| error.to_string())
        }))
    }
    pub(super) fn set_right_stick(
        &mut self,
        x: DualSenseAxis,
        y: DualSenseAxis,
    ) -> Result<(), String> {
        self.queue(Box::new(move |controller| {
            let Controller::DualSense(controller) = controller else {
                return Err("edit family mismatch".into());
            };
            controller
                .set_right_stick(x, y)
                .map_err(|error| error.to_string())
        }))
    }
    pub(super) fn set_triggers(
        &mut self,
        left: DualSenseTrigger,
        right: DualSenseTrigger,
    ) -> Result<(), String> {
        self.queue(Box::new(move |controller| {
            let Controller::DualSense(controller) = controller else {
                return Err("edit family mismatch".into());
            };
            controller
                .set_triggers(left, right)
                .map_err(|error| error.to_string())
        }))
    }
    pub(super) fn set_touch(
        &mut self,
        slot: TouchSlot,
        contact: Option<DualSenseTouchContact>,
    ) -> Result<(), String> {
        self.queue(Box::new(move |controller| {
            let Controller::DualSense(controller) = controller else {
                return Err("edit family mismatch".into());
            };
            controller
                .set_touch(slot, contact)
                .map_err(|error| error.to_string())
        }))
    }
    pub(super) fn set_motion(&mut self, motion: MotionSample) -> Result<(), String> {
        self.queue(Box::new(move |controller| {
            let Controller::DualSense(controller) = controller else {
                return Err("edit family mismatch".into());
            };
            controller
                .set_motion(motion)
                .map_err(|error| error.to_string())
        }))
    }
    pub(super) fn set_battery_exposed(&mut self, exposed: bool) -> Result<(), String> {
        self.queue(Box::new(move |controller| {
            let Controller::DualSense(controller) = controller else {
                return Err("edit family mismatch".into());
            };
            controller
                .set_battery_exposed(exposed)
                .map_err(|error| error.to_string())
        }))
    }
    pub(super) fn set_battery_level(&mut self, level: BatteryLevel) -> Result<(), String> {
        self.queue(Box::new(move |controller| {
            let Controller::DualSense(controller) = controller else {
                return Err("edit family mismatch".into());
            };
            controller
                .set_battery_level(level)
                .map_err(|error| error.to_string())
        }))
    }
}
impl DualShock4Editor {
    pub(super) fn set_digital(&mut self, update: DigitalControlUpdate) -> Result<(), String> {
        self.queue(Box::new(move |controller| {
            let Controller::DualShock4(controller) = controller else {
                return Err("edit family mismatch".into());
            };
            controller
                .set_digital(update)
                .map_err(|error| error.to_string())
        }))
    }
    pub(super) fn set_native(
        &mut self,
        control: DualShock4Control,
        pressed: bool,
    ) -> Result<(), String> {
        self.queue(Box::new(move |controller| {
            let Controller::DualShock4(controller) = controller else {
                return Err("edit family mismatch".into());
            };
            controller
                .set_native(control, pressed)
                .map_err(|error| error.to_string())
        }))
    }
    pub(super) fn set_left_stick(
        &mut self,
        x: DualShock4Axis,
        y: DualShock4Axis,
    ) -> Result<(), String> {
        self.queue(Box::new(move |controller| {
            let Controller::DualShock4(controller) = controller else {
                return Err("edit family mismatch".into());
            };
            controller
                .set_left_stick(x, y)
                .map_err(|error| error.to_string())
        }))
    }
    pub(super) fn set_right_stick(
        &mut self,
        x: DualShock4Axis,
        y: DualShock4Axis,
    ) -> Result<(), String> {
        self.queue(Box::new(move |controller| {
            let Controller::DualShock4(controller) = controller else {
                return Err("edit family mismatch".into());
            };
            controller
                .set_right_stick(x, y)
                .map_err(|error| error.to_string())
        }))
    }
    pub(super) fn set_triggers(
        &mut self,
        left: DualShock4Trigger,
        right: DualShock4Trigger,
    ) -> Result<(), String> {
        self.queue(Box::new(move |controller| {
            let Controller::DualShock4(controller) = controller else {
                return Err("edit family mismatch".into());
            };
            controller
                .set_triggers(left, right)
                .map_err(|error| error.to_string())
        }))
    }
    pub(super) fn set_touch(
        &mut self,
        slot: DualShock4TouchSlot,
        contact: Option<DualShock4TouchContact>,
    ) -> Result<(), String> {
        self.queue(Box::new(move |controller| {
            let Controller::DualShock4(controller) = controller else {
                return Err("edit family mismatch".into());
            };
            controller
                .set_touch(slot, contact)
                .map_err(|error| error.to_string())
        }))
    }
    pub(super) fn set_motion(&mut self, motion: DualShock4MotionSample) -> Result<(), String> {
        self.queue(Box::new(move |controller| {
            let Controller::DualShock4(controller) = controller else {
                return Err("edit family mismatch".into());
            };
            controller
                .set_motion(motion)
                .map_err(|error| error.to_string())
        }))
    }
}
impl SwitchProEditor {
    pub(super) fn set_digital(&mut self, update: DigitalControlUpdate) -> Result<(), String> {
        self.queue(Box::new(move |controller| {
            let Controller::SwitchPro(controller) = controller else {
                return Err("edit family mismatch".into());
            };
            controller
                .set_digital(update)
                .map_err(|error| error.to_string())
        }))
    }
    pub(super) fn set_native(
        &mut self,
        control: SwitchProControl,
        pressed: bool,
    ) -> Result<(), String> {
        self.queue(Box::new(move |controller| {
            let Controller::SwitchPro(controller) = controller else {
                return Err("edit family mismatch".into());
            };
            controller
                .set_native(control, pressed)
                .map_err(|error| error.to_string())
        }))
    }
    pub(super) fn set_left_stick(
        &mut self,
        x: SwitchProAxis,
        y: SwitchProAxis,
    ) -> Result<(), String> {
        self.queue(Box::new(move |controller| {
            let Controller::SwitchPro(controller) = controller else {
                return Err("edit family mismatch".into());
            };
            controller
                .set_left_stick(x, y)
                .map_err(|error| error.to_string())
        }))
    }
    pub(super) fn set_right_stick(
        &mut self,
        x: SwitchProAxis,
        y: SwitchProAxis,
    ) -> Result<(), String> {
        self.queue(Box::new(move |controller| {
            let Controller::SwitchPro(controller) = controller else {
                return Err("edit family mismatch".into());
            };
            controller
                .set_right_stick(x, y)
                .map_err(|error| error.to_string())
        }))
    }
    pub(super) fn set_motion(&mut self, motion: SwitchProMotionSample) -> Result<(), String> {
        self.queue(Box::new(move |controller| {
            let Controller::SwitchPro(controller) = controller else {
                return Err("edit family mismatch".into());
            };
            controller
                .set_motion(motion)
                .map_err(|error| error.to_string())
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn w6_inputs_after_native_attach_failure_use_retained_topology() {
        use gr_curated_controllers::wii_remote::{WiiRemoteButton, WiiRemoteState};
        let ControllerView::WiiRemote(mut editor) = neutral_view(super::super::Kind::WiiRemote)
        else {
            unreachable!()
        };
        editor.select_wii_attachment(WiiExtensionChoice::Nunchuk, true);
        let accessory = super::super::InputEvent::Button {
            id: virtualgamepad::InputControlId::new("nunchuk.c"),
            pressed: true,
        };
        editor.edit_wii(&accessory);
        assert!(
            matches!(editor.state.extension(), gr_curated_controllers::wii_remote::WiiExtension::Nunchuk(n) if n.c)
        );
        // A worker attach failure retains the old state. A later accessory event
        // cannot restore the optimistic shape; core events still work afterward.
        let retained = WiiRemoteState::default();
        assert!(prepare_wii_input(&retained, &accessory).is_err());
        let core = super::super::InputEvent::Button {
            id: virtualgamepad::InputControlId::new("a"),
            pressed: true,
        };
        let pressed = prepare_wii_input(&retained, &core).unwrap();
        assert!(pressed.pressed(WiiRemoteButton::A));
        assert_eq!(pressed.extension(), retained.extension());
        assert_eq!(pressed.motion_plus(), retained.motion_plus());
        let released = prepare_wii_input(
            &pressed,
            &super::super::InputEvent::Button {
                id: virtualgamepad::InputControlId::new("a"),
                pressed: false,
            },
        )
        .unwrap();
        assert_eq!(released, retained);
    }
    #[test]
    fn w6_classic_pro_selection_round_trips_without_becoming_classic() {
        for choice in WiiExtensionChoice::ALL {
            assert_eq!(
                WiiExtensionChoice::from_extension(&choice.extension().unwrap()),
                choice
            );
        }
    }
    #[test]
    fn w6_rejected_attachment_keeps_selection_and_topology_and_logs_failure() {
        let ControllerView::WiiRemote(mut editor) = neutral_view(super::super::Kind::WiiRemote)
        else {
            unreachable!()
        };
        editor.select_wii_attachment(WiiExtensionChoice::Nunchuk, true);
        let before = editor.state.clone();
        let topology = editor.state.input_topology();
        editor.select_wii_attachment(WiiExtensionChoice::ClassicPro, false);
        assert_eq!(editor.state, before);
        assert_eq!(editor.state.input_topology(), topology);
        assert_eq!(
            WiiExtensionChoice::from_extension(editor.state.extension()),
            WiiExtensionChoice::Nunchuk
        );
        let feedback = editor.feedback.lock().unwrap();
        assert!(feedback.error.as_ref().unwrap().contains("Attachment"));
        assert_eq!(feedback.diagnostics.len(), 1);
        assert!(!feedback.diagnostics[0].success);
    }
    #[test]
    fn w6_full_batch_rejects_attachment_and_ir_without_poisoning_pending_edits() {
        let ControllerView::WiiRemote(mut editor) = neutral_view(super::super::Kind::WiiRemote)
        else {
            unreachable!()
        };
        for _ in 0..EDIT_LIMIT {
            editor.queue(Box::new(|_| Ok(()))).unwrap();
        }
        let before = editor.state.clone();
        editor.select_wii_attachment(WiiExtensionChoice::Nunchuk, true);
        editor.edit_wii_ir([
            Some(gr_curated_controllers::wii_remote::WiiIrPoint::new(12, 34).unwrap()),
            None,
            None,
            None,
        ]);
        assert_eq!(editor.state, before);
        assert_eq!(editor.take_edits().unwrap().len(), EDIT_LIMIT);
        assert_eq!(editor.feedback.lock().unwrap().diagnostics.len(), 2);
    }
    #[test]
    fn w6_native_rejection_survives_snapshot_and_diagnostic_drain() {
        let mut old = neutral_view(super::super::Kind::WiiRemote);
        let ControllerView::WiiRemote(editor) = &mut old else {
            unreachable!()
        };
        record_wii_result(&editor.feedback, "IR", &Err("rejected".into()));
        let mut next = neutral_view(super::super::Kind::WiiRemote);
        next.preserve_editor_feedback(&old);
        let diagnostics = next.take_diagnostics();
        assert_eq!(diagnostics.len(), 1);
        assert!(!diagnostics[0].success);
        assert!(next.take_diagnostics().is_empty());
        let ControllerView::WiiRemote(editor) = next else {
            unreachable!()
        };
        assert_eq!(
            editor.feedback.lock().unwrap().error.as_deref(),
            Some("IR: rejected")
        );
    }
    #[test]
    fn wii_attachment_projection_precedes_same_frame_native_edits() {
        use gr_curated_controllers::wii_remote::{WiiClassicState, WiiExtension, WiiNunchukState};
        let ControllerView::WiiRemote(mut editor) = neutral_view(super::super::Kind::WiiRemote)
        else {
            unreachable!()
        };
        editor
            .attach_wii(WiiExtension::Nunchuk(WiiNunchukState::default()), true)
            .unwrap();
        assert!(matches!(editor.state.extension(), WiiExtension::Nunchuk(_)));
        assert!(editor.state.motion_plus().attached);
        editor.edit_wii(&super::super::InputEvent::Axis1 {
            id: virtualgamepad::InputControlId::new("nunchuk.accel.x"),
            value: 321,
        });
        editor.edit_wii(&super::super::InputEvent::Axis1 {
            id: virtualgamepad::InputControlId::new("gyro.x"),
            value: 1234,
        });
        let WiiExtension::Nunchuk(nunchuk) = editor.state.extension() else {
            unreachable!()
        };
        assert_eq!(nunchuk.accelerometer[0], 321);
        assert_eq!(editor.state.motion_plus().gyroscope[0], 1234);
        assert_eq!(editor.edits.len(), 3);
        // Rejected attachments and a full queue must not change the projection.
        let before = editor.state.clone();
        assert!(
            editor
                .attach_wii(WiiExtension::ClassicPro(WiiClassicState::default()), false)
                .is_err()
        );
        for _ in editor.edits.len()..EDIT_LIMIT {
            editor.queue(Box::new(|_| Ok(()))).unwrap();
        }
        assert!(editor.attach_wii(WiiExtension::None, false).is_err());
        assert_eq!(editor.state, before);
    }
    #[test]
    fn detached_editor_bounds_work_and_never_changes_its_native_snapshot() {
        let mut editor = Editor {
            state: Xbox360State::default(),
            metadata: (),
            surface: &(),
            edits: Vec::new(),
            overflow: false,
            diagnostics: None,
            association: None,
            feedback: std::sync::Arc::default(),
        };
        let original = editor.state.clone();
        for _ in 0..EDIT_LIMIT {
            editor.queue(Box::new(|_| Ok(()))).unwrap();
        }
        assert_eq!(editor.state, original);
        assert_eq!(editor.edits.len(), EDIT_LIMIT);
        assert!(editor.queue(Box::new(|_| Ok(()))).is_err());
        assert!(editor.take_edits().is_err());
        assert_eq!(editor.edits.len(), EDIT_LIMIT);
    }
}

impl SteamEditor {
    pub(super) fn edit_steam(&mut self, events: Vec<super::InputEvent>) {
        let mut next = self.state.clone();
        for event in events {
            if !super::native_editor::apply_steam_event(&mut next, &event) {
                return;
            }
        }
        self.queue_steam_state(next);
    }
    fn queue_steam_state(
        &mut self,
        next: gr_curated_controllers::steam_controller_2026::SteamController2026State,
    ) {
        if next == self.state {
            return;
        }
        let queued = next.clone();
        if self
            .queue(Box::new(move |c| {
                let Controller::Steam(c) = c else {
                    return Err("edit family mismatch".into());
                };
                c.set_inputs(queued).map_err(|e| e.to_string())
            }))
            .is_ok()
        {
            self.state = next;
        }
    }
}

impl SteamEditor {
    pub(super) fn draw_steam(&mut self, ui: &mut super::egui::Ui) {
        let mut next = self.state.clone();
        let mut changed = false;
        for i in 0..2 {
            ui.horizontal(|ui| {
                ui.label(if i == 0 { "Left" } else { "Right" });
                changed |= ui
                    .checkbox(&mut next.stick_touch[i], "Stick touch")
                    .changed();
                changed |= ui.checkbox(&mut next.grip_touch[i], "Grip touch").changed();
                changed |= ui.checkbox(&mut next.pads[i].touch, "Pad touch").changed();
                changed |= ui
                    .add(
                        super::egui::Slider::new(&mut next.pads[i].pressure, 0..=u16::MAX)
                            .text("Pad pressure"),
                    )
                    .changed();
            });
        }
        ui.horizontal(|ui| {
            ui.label("Raw motion timestamp (microseconds)");
            changed |= ui
                .add(super::egui::DragValue::new(&mut next.motion.timestamp_us))
                .changed();
        });
        if changed {
            self.queue_steam_state(next);
        }
    }
}

#[cfg(test)]
mod steam_tests {
    use super::super::{InputEvent, Kind};
    use super::*;
    use virtualgamepad::InputControlId as Id;
    #[test]
    fn steam_native_edits_cover_extrema_contacts_release_and_atomic_rejection() {
        use super::super::native_editor::{InputEditor, apply_steam_event};
        let ControllerView::Steam(mut e) = neutral_view(Kind::Steam) else {
            unreachable!()
        };
        for (id, i) in [("left_stick", 0), ("right_stick", 1)] {
            e.apply_events(vec![InputEvent::Axis2 {
                id: Id::new(id),
                x: -32768,
                y: 32767,
            }]);
            assert_eq!(e.state.sticks[i], [-32768, 32767]);
        }
        for (id, i) in [("left_pad", 0), ("right_pad", 1)] {
            e.apply_events(vec![InputEvent::Touch {
                id: Id::new(id),
                contact: 0,
                point: Some((0, 65535)),
            }]);
            assert_eq!(e.state.pads[i].position, [-32768, 32767]);
            assert!(e.state.pads[i].touch);
            e.apply_events(vec![InputEvent::Touch {
                id: Id::new(id),
                contact: 0,
                point: Some((12345, 54321)),
            }]);
            assert_eq!(e.state.pads[i].position, [-20423, 21553]);

            e.apply_events(vec![InputEvent::Touch {
                id: Id::new(id),
                contact: 0,
                point: None,
            }]);
            assert!(!e.state.pads[i].touch);
        }
        for id in ["left_trigger.axis", "right_trigger.axis"] {
            e.apply_events(vec![InputEvent::Axis1 {
                id: Id::new(id),
                value: 32767,
            }]);
        }
        assert_eq!(e.state.triggers, [32767; 2]);
        e.apply_events(vec![InputEvent::Motion {
            id: Id::new("imu"),
            gyroscope: [-32768, 0, 32767],
            accelerometer: [32767, 0, -32768],
        }]);
        assert_eq!(e.state.motion.gyroscope, [-32768, 0, 32767]);
        let before = e.state.clone();
        assert!(!apply_steam_event(
            &mut e.state,
            &InputEvent::Axis2 {
                id: Id::new("left_stick"),
                x: 32768,
                y: 0
            }
        ));
        assert_eq!(e.state, before);
        while e.edits.len() < EDIT_LIMIT {
            e.queue(Box::new(|_| Ok(()))).unwrap();
        }
        e.apply_events(vec![InputEvent::Axis1 {
            id: Id::new("left_trigger.axis"),
            value: 0,
        }]);
        assert_eq!(e.state, before);
        assert!(e.take_edits().is_err());
    }
}
