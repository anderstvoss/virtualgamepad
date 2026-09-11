use virtualgamepad::{DualSenseTouchContact, TouchSlot, Xbox360Controller};
fn invalid(controller: &mut Xbox360Controller, contact: DualSenseTouchContact) {
    controller.set_touch(TouchSlot::First, Some(contact));
}
fn main() {}
