use gr_controller_contract::{prepare_deployment_realization, RealizationControllerDefinition};
use gr_realization_api::TransportValidationTarget;

fn prove_target_types_do_not_mix(controller: &dyn RealizationControllerDefinition) {
    let _ = prepare_deployment_realization(controller, TransportValidationTarget::UsbGadget);
}

fn main() {}
