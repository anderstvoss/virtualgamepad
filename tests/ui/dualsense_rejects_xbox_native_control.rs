use virtualgamepad::{DualSenseController, XboxControl};

fn invalid(controller: &mut DualSenseController) {
    controller.set_native(XboxControl::A, true);
}

fn main() {}
