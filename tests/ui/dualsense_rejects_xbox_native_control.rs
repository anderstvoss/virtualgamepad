use virtualgamepad::{DualSenseController, Xbox360Control};
fn invalid(controller: &mut DualSenseController) {
    controller.set_native(Xbox360Control::A, true);
}
fn main() {}
