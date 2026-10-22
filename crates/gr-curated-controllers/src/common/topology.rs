//! Opt-in retained live shape. Ordinary `ControllerSession` has no added fields.
use super::{ControllerSession, HidDriver};
use gr_controller_contract::{ControlError, InputControlId, InputTopology};
use gr_realization_api::{RealizationTarget, RealizationTargetSet};

/// A compiled controller operation derives its ID; callers cannot authorize an
/// unrelated arbitrary closure by supplying a second, independent ID.
pub(crate) trait ControlEdit<S> {
    fn control_id(&self) -> InputControlId;
    fn apply(self, state: &mut S) -> Result<(), ControlError>;
}

pub(crate) struct LiveTopology<P: gr_hid::Protocol> {
    maximum: &'static InputTopology,
    current: &'static InputTopology,
    generation: u64,
    native_protocol: Option<P>,
}
impl<P: gr_hid::Protocol> LiveTopology<P> {
    pub(crate) fn new(
        maximum: &'static InputTopology,
        current: &'static InputTopology,
        native_protocol: Option<P>,
    ) -> Result<Self, ControlError> {
        maximum.validate().map_err(|_| invalid())?;
        validate_subset(maximum, current)?;
        Ok(Self {
            maximum,
            current,
            generation: 0,
            native_protocol,
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
    pub(crate) fn change<D: HidDriver<Hid = P, State = P::State>>(
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
        // Reinitialization can change controller configuration without changing
        // its shape. Always validate/install it; only shape changes count.
        let generation = if self.current == next {
            self.generation
        } else {
            self.generation.checked_add(1).ok_or_else(invalid)?
        };
        if let Some(protocol) = &mut self.native_protocol {
            session.update_native_configuration(protocol, neutralize_removed, edit_protocol)?;
        } else {
            session.update_configuration(neutralize_removed, edit_protocol)?;
        }
        self.current = next;
        self.generation = generation;
        Ok(())
    }
    pub(crate) fn edit<D: HidDriver<Hid = P, State = P::State>>(
        &self,
        session: &mut ControllerSession<D>,
        edit: impl ControlEdit<D::State>,
    ) -> Result<(), ControlError> {
        if session.is_closed() {
            return Err(ControlError::Closed);
        }
        if !contains(self.current, edit.control_id()) {
            return Err(unavailable(session.selection().target));
        }
        session.update_state(|state| {
            edit.apply(state)?;
            if let Some(protocol) = &self.native_protocol {
                protocol.validate(state).map_err(|_| invalid())?;
            }
            Ok(())
        })
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
        custom_modules,
        typed_inputs
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
        || topology.typed_inputs().iter().any(|v| v.id() == id)
        || topology.custom_modules().iter().any(|v| v.id == id)
}

#[cfg(test)]
mod typed_input_tests {
    use super::*;
    use gr_controller_contract::{
        InputKind, TypedInput,
        construction::{InputTopologySpec, TypedInputSpec},
    };
    const ID: InputControlId = InputControlId::new("synthetic.contact");
    static CONTACT: &[TypedInput] = &[TypedInputSpec {
        id: ID,
        title: "Contact",
        kind: InputKind::CapacitiveTouch,
    }
    .build()];
    static SHAPE: InputTopology = InputTopologySpec {
        typed_inputs: CONTACT,
        ..InputTopologySpec::from_descriptor(InputTopology::EMPTY)
    }
    .build();
    #[test]
    fn typed_inputs_participate_in_presence_and_immutable_subset_validation() {
        static REDEFINED: &[TypedInput] = &[TypedInputSpec {
            id: ID,
            title: "Different meaning",
            kind: InputKind::CapacitiveTouch,
        }
        .build()];
        assert!(!contains(&InputTopology::EMPTY, ID));
        assert!(contains(&SHAPE, ID));
        assert!(validate_subset(&SHAPE, &InputTopology::EMPTY).is_ok());
        assert!(validate_subset(&InputTopology::EMPTY, &SHAPE).is_err());
        let redefined = InputTopologySpec {
            typed_inputs: REDEFINED,
            ..InputTopologySpec::from_descriptor(InputTopology::EMPTY)
        }
        .build();
        assert!(validate_subset(&SHAPE, &redefined).is_err());
    }
}
