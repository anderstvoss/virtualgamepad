//! Detached native-state views and bounded edits for the demo only.
use super::{
    BatteryLevel, Controller, DigitalControlUpdate, DualSenseAxis, DualSenseControl,
    DualSenseTouchContact, DualSenseTrigger, DualShock4Axis, DualShock4Control,
    DualShock4MotionSample, DualShock4TouchContact, DualShock4TouchSlot, DualShock4Trigger,
    MotionSample, SwitchProAxis, SwitchProControl, SwitchProMotionSample, TouchSlot, Xbox360Axis,
    Xbox360Control, Xbox360Trigger,
};
use virtualgamepad::{
    ControllerDiagnostics, DualSenseState, DualShock4State, SwitchProState, Xbox360State,
};

pub(super) const EDIT_LIMIT: usize = 64;
pub(super) type Command<C> = Box<dyn FnOnce(&mut C) -> Result<(), String> + Send>;
type Edit = Command<Controller>;
pub(super) type EditBatch = Vec<Edit>;

pub(super) struct Editor<S, T: 'static> {
    state: S,
    surface: T,
    edits: EditBatch,
    overflow: bool,
    diagnostics: Option<ControllerDiagnostics>,
    association: Option<virtualgamepad::ControllerAssociation>,
}
impl<S, T> Editor<S, T> {
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
pub(super) type Xbox360Editor = Editor<Xbox360State, Surface>;
pub(super) type DualSenseEditor = Editor<DualSenseState, Surface>;
pub(super) type DualShock4Editor = Editor<DualShock4State, Surface>;
pub(super) type SwitchProEditor = Editor<SwitchProState, Surface>;
pub(super) enum ControllerView {
    Xbox(Xbox360Editor),
    DualSense(DualSenseEditor),
    DualShock4(DualShock4Editor),
    SwitchPro(SwitchProEditor),
}
impl<S> Editor<S, Surface> {
    fn new(
        state: S,
        surface: &'static virtualgamepad::ControllerSurface,
        diagnostics: Option<ControllerDiagnostics>,
        association: Option<virtualgamepad::ControllerAssociation>,
    ) -> Self {
        Self {
            state,
            surface: Surface(surface),
            edits: Vec::new(),
            overflow: false,
            diagnostics,
            association,
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
                ))
            };
        }
        match self {
            Self::Xbox(c) => view!(c, Xbox),
            Self::DualSense(c) => view!(c, DualSense),
            Self::DualShock4(c) => view!(c, DualShock4),
            Self::SwitchPro(c) => view!(c, SwitchPro),
        }
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
        super::Kind::Xbox360 => {
            ControllerView::Xbox(Editor::new(Xbox360State::default(), surface, None, None))
        }
        super::Kind::DualSense => {
            ControllerView::DualSense(Editor::new(DualSenseState::default(), surface, None, None))
        }
        super::Kind::DualShock4 => {
            ControllerView::DualShock4(Editor::new(DualShock4State::default(), surface, None, None))
        }
        super::Kind::SwitchPro => {
            ControllerView::SwitchPro(Editor::new(SwitchProState::default(), surface, None, None))
        }
    }
}
impl ControllerView {
    pub(super) fn association(&self) -> Option<&virtualgamepad::ControllerAssociation> {
        match self {
            Self::Xbox(c) => c.association.as_ref(),
            Self::DualSense(c) => c.association.as_ref(),
            Self::DualShock4(c) => c.association.as_ref(),
            Self::SwitchPro(c) => c.association.as_ref(),
        }
    }
    pub(super) fn lab_details(&self) -> String {
        match self {
            Self::Xbox(c) => format!("{:?}; {:?}", c.association, c.diagnostics),
            Self::DualSense(c) => format!("{:?}; {:?}", c.association, c.diagnostics),
            Self::DualShock4(c) => format!("{:?}; {:?}", c.association, c.diagnostics),
            Self::SwitchPro(c) => format!("{:?}; {:?}", c.association, c.diagnostics),
        }
    }
    pub(super) fn release_inputs(&mut self) -> Result<(), String> {
        let edit = super::release_inputs();
        match self {
            Self::Xbox(c) => c.queue(edit),
            Self::DualSense(c) => c.queue(edit),
            Self::DualShock4(c) => c.queue(edit),
            Self::SwitchPro(c) => c.queue(edit),
        }
    }

    pub(super) fn take_edits(&mut self) -> Result<EditBatch, String> {
        match self {
            Self::Xbox(c) => c.take_edits(),
            Self::DualSense(c) => c.take_edits(),
            Self::DualShock4(c) => c.take_edits(),
            Self::SwitchPro(c) => c.take_edits(),
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
    fn detached_editor_bounds_work_and_never_changes_its_native_snapshot() {
        let mut editor = Editor {
            state: Xbox360State::default(),
            surface: &(),
            edits: Vec::new(),
            overflow: false,
            diagnostics: None,
            association: None,
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
