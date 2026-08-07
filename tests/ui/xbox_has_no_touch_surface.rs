use virtualgamepad::{DualSenseTouchContact, Xbox360Controller};

fn invalid(controller: &mut Xbox360Controller) {
    controller.set_touch_contact(0, DualSenseTouchContact::neutral());
}

fn main() {}
