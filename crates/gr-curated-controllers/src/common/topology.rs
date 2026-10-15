//! Opt-in retained live shape. Ordinary `ControllerSession` has no added fields.
use super::{ControllerSession, HidDriver};
use gr_controller_contract::{ControlError, InputControlId, InputTopology};
use gr_realization_api::{RealizationTarget, RealizationTargetSet};

#[derive(Clone, Copy)]
pub(crate) struct LiveTopology {
    maximum: &'static InputTopology,
    current: &'static InputTopology,
    generation: u64,
}
impl LiveTopology {
    pub(crate) fn new(
        maximum: &'static InputTopology,
        current: &'static InputTopology,
    ) -> Result<Self, ControlError> {
        maximum.validate().map_err(|_| invalid())?;
        validate_subset(maximum, current)?;
        Ok(Self {
            maximum,
            current,
            generation: 0,
        })
    }
    pub(crate) const fn current_topology(&self) -> &'static InputTopology {
        self.current
    }
    pub(crate) const fn topology_generation(&self) -> u64 {
        self.generation
    }
    /// Validate the shape/counter before running a typed, transactional protocol edit.
    /// Callers retain controller-native attachment rules and bounded status edges.
    pub(crate) fn change<D: HidDriver>(
        &mut self,
        session: &mut ControllerSession<D>,
        next: &'static InputTopology,
        neutralize_removed: impl FnOnce(&mut D::State) -> Result<(), ControlError>,
        edit_protocol: impl FnOnce(&mut D::Hid) -> Result<(), gr_hid::Error>,
    ) -> Result<(), ControlError> {
        if session.is_closed() {
            return Err(ControlError::Closed);
        }
        validate_subset(self.maximum, next)?;
        if self.current == next {
            return Ok(());
        }
        let generation = self.generation.checked_add(1).ok_or_else(invalid)?;
        session.update_configuration(neutralize_removed, edit_protocol)?;
        self.current = next;
        self.generation = generation;
        Ok(())
    }
    pub(crate) fn edit<D: HidDriver>(
        &self,
        session: &mut ControllerSession<D>,
        id: InputControlId,
        edit: impl FnOnce(&mut D::State) -> Result<(), ControlError>,
    ) -> Result<(), ControlError> {
        if session.is_closed() {
            return Err(ControlError::Closed);
        }
        if !contains(self.current, id) {
            return Err(unavailable(session.selection().target));
        }
        session.update_state(edit)
    }
}
fn invalid() -> ControlError {
    ControlError::UnsupportedControl {
        control: "invalid live topology transition",
    }
}
fn unavailable(target: RealizationTarget) -> ControlError {
    ControlError::UnavailableInRealization {
        selected_target: target,
        available_in: RealizationTargetSet::EMPTY,
    }
}
fn validate_subset(maximum: &InputTopology, current: &InputTopology) -> Result<(), ControlError> {
    current.validate().map_err(|_| invalid())?;
    // Entries carry immutable semantic meanings/ranges; a live shape may remove
    // entire entries but cannot redefine a control already in the maximum.
    macro_rules! subset { ($($kind:ident),+) => { $(if !current.$kind().iter().all(|entry| maximum.$kind().contains(entry)) { return Err(invalid()); })+ }; }
    subset!(
        auxiliary_buttons,
        face_button_clusters,
        dpads,
        sticks,
        trigger_stacks,
        touchpads,
        motion,
        extra_axes,
        custom_modules
    );
    Ok(())
}
fn contains(topology: &InputTopology, id: InputControlId) -> bool {
    topology.auxiliary_buttons().iter().any(|v| v.id == id)
        || topology.face_button_clusters().iter().any(|v| v.id() == id)
        || topology.dpads().iter().any(|v| v.id() == id)
        || topology.sticks().iter().any(|v| {
            v.id() == id
                || v.press().is_some_and(|v| v.id == id)
                || v.capacitive().is_some_and(|v| v.id == id)
        })
        || topology.trigger_stacks().iter().any(|v| {
            v.id() == id
                || v.controls().iter().any(|control| match control.kind {
                    gr_controller_contract::TriggerInputKind::Button { id: other }
                    | gr_controller_contract::TriggerInputKind::Axis { id: other, .. } => {
                        other == id
                    }
                })
        })
        || topology.touchpads().iter().any(|v| {
            v.id() == id
                || match v.actuation() {
                    gr_controller_contract::TouchpadActuation::Button(button) => button.id == id,
                    gr_controller_contract::TouchpadActuation::Deflection { id: other, .. } => {
                        other == id
                    }
                    gr_controller_contract::TouchpadActuation::None => false,
                }
        })
        || topology.motion().iter().any(|v| v.id() == id)
        || topology.extra_axes().iter().any(|v| match v {
            gr_controller_contract::ExtraAxisInput::OneDimensional { id: other, .. }
            | gr_controller_contract::ExtraAxisInput::TwoDimensional { id: other, .. } => {
                *other == id
            }
        })
        || topology.custom_modules().iter().any(|v| v.id == id)
}
