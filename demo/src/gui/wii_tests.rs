//! Headless integration coverage for the Wii demo adapter and generic renderer.
use super::{
    editor::{self, ControllerView, WiiExtensionChoice},
    input_clusters::{self, InputUiState},
    native_editor::InputEditor,
    *,
};
use virtualgamepad::InputControlId;

fn text_shapes(shape: &egui::epaint::Shape, out: &mut Vec<(String, egui::Rect)>) {
    match shape {
        egui::epaint::Shape::Text(text) => out.push((
            text.galley.job.text.clone(),
            text.galley.rect.translate(text.pos.to_vec2()),
        )),
        egui::epaint::Shape::Vec(shapes) => {
            for shape in shapes {
                text_shapes(shape, out);
            }
        }
        _ => {}
    }
}
fn render(mut draw: impl FnMut(&mut egui::Ui)) -> Vec<(String, egui::Rect)> {
    let context = egui::Context::default();
    let output = context.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                Pos2::ZERO,
                Vec2::new(1500.0, 2400.0),
            )),
            ..Default::default()
        },
        |context| {
            egui::CentralPanel::default().show(context, |ui| draw(ui));
        },
    );
    let mut texts = Vec::new();
    for shape in &output.shapes {
        text_shapes(&shape.shape, &mut texts);
    }
    texts
}
#[test]
fn attachment_card_is_in_the_row_and_covers_only_known_native_modules() {
    let ControllerView::WiiRemote(mut view) = editor::neutral_view(Kind::WiiRemote) else {
        unreachable!()
    };
    view.select_wii_attachment(WiiExtensionChoice::Nunchuk, true);
    let mut row = egui::Rect::NOTHING;
    let labels = render(|ui| {
        row = view.draw_wii(ui, 7);
        input_clusters::draw_editor(ui, 7, &mut view, &mut InputUiState::default());
    });
    let heading = labels
        .iter()
        .find(|(label, _)| label == "Attachment")
        .unwrap();
    assert!(row.contains_rect(heading.1));
    for text in [
        "None",
        "Nunchuk",
        "Classic",
        "Classic Pro",
        "MotionPlus bridge",
        "Topology: Wii Remote + MotionPlus + Nunchuk",
    ] {
        assert!(
            labels.iter().any(|(label, _)| label == text),
            "missing {text}"
        );
    }
    assert!(
        !labels
            .iter()
            .any(|(label, _)| label.contains("custom module requires"))
    );
    assert!(!view.handles_module(virtualgamepad::InputControlId::new("unknown-accessory")));
}
#[test]
fn attachment_choices_drive_six_live_shapes_and_retain_classic_pro_refusals() {
    let ControllerView::WiiRemote(mut view) = editor::neutral_view(Kind::WiiRemote) else {
        unreachable!()
    };
    for choice in WiiExtensionChoice::ALL {
        for bridge in [false, true] {
            let before = view.state().clone();
            view.select_wii_attachment(choice, bridge);
            if choice == WiiExtensionChoice::ClassicPro {
                assert_eq!(view.state(), &before);
                continue;
            }
            assert_eq!(
                WiiExtensionChoice::from_extension(view.state().extension()),
                choice
            );
            assert_eq!(view.state().motion_plus().attached, bridge);
            let modules: Vec<_> = view
                .current_topology()
                .custom_modules()
                .iter()
                .map(|m| m.id.as_str())
                .collect();
            assert_eq!(modules.contains(&"motion-plus"), bridge);
            assert_eq!(
                modules.contains(&"nunchuk"),
                choice == WiiExtensionChoice::Nunchuk
            );
            assert_eq!(
                modules.contains(&"classic"),
                choice == WiiExtensionChoice::Classic
            );
        }
    }
}
#[test]
fn unknown_modules_keep_the_explicit_placeholder() {
    use virtualgamepad::{CustomInputModule, InputControlId, InputTopology};
    static TOPOLOGY: InputTopology = gr_controller_contract::construction::InputTopologySpec {
        custom_modules: &[CustomInputModule {
            id: InputControlId::new("unknown-accessory"),
            title: "Unknown accessory",
        }],
        ..gr_controller_contract::construction::InputTopologySpec::from_descriptor(
            InputTopology::EMPTY,
        )
    }
    .build();
    struct Unknown;
    impl InputEditor for Unknown {
        fn current_topology(&self) -> &'static InputTopology {
            &TOPOLOGY
        }
        fn stick_value(&self, _: InputControlId) -> Option<(i32, i32)> {
            None
        }
        fn trigger_values(&self) -> Vec<(InputControlId, InputValue)> {
            vec![]
        }
        fn apply_events(&mut self, _: Vec<InputEvent>) {}
    }
    let labels =
        render(|ui| input_clusters::draw_editor(ui, 1, &mut Unknown, &mut InputUiState::default()));
    assert!(
        labels.iter().any(|(label, _)| label
            == "Unknown accessory: custom module requires a declared native editor")
    );
}

#[test]
fn revision_is_wii_only_defaults_original_and_state_grid_uses_actual_revision() {
    assert_eq!(App::default().wii_revision, WiiRemoteRevision::Original);
    let mut revision = WiiRemoteRevision::Original;
    let labels = render(|ui| draw_wii_revision_selector(ui, Kind::Xbox360, &mut revision));
    assert!(!labels.iter().any(|(text, _)| text == "Wii revision"));
    for choice in [
        WiiRemoteRevision::Original,
        WiiRemoteRevision::EarlyIntegratedPlus,
        WiiRemoteRevision::Tr,
    ] {
        revision = choice;
        let view = editor::neutral_wii_view(choice);
        let labels = render(|ui| {
            draw_wii_revision_selector(ui, Kind::WiiRemote, &mut revision);
            egui::Grid::new("revision-state").show(ui, |ui| draw_wii_revision_state(ui, &view));
        });
        assert!(labels.iter().any(|(text, _)| text == "Revision"));
        assert!(
            labels
                .iter()
                .any(|(text, _)| text == wii_revision_label(choice))
        );
        assert_eq!(
            labels
                .iter()
                .any(|(text, _)| text.contains("no identity evidence")),
            choice == WiiRemoteRevision::EarlyIntegratedPlus
        );
    }
    assert!(
        validate_wii_recreation(Kind::WiiRemote, WiiRemoteRevision::EarlyIntegratedPlus).is_err()
    );
    assert!(validate_wii_recreation(Kind::WiiRemote, WiiRemoteRevision::Original).is_ok());
    assert!(
        validate_wii_recreation(Kind::DualSense, WiiRemoteRevision::EarlyIntegratedPlus).is_ok()
    );
}

#[test]
fn wii_output_snapshots_update_four_leds_and_rumble_without_sony_rows() {
    let mut indicators = ReverseIndicators::default();
    for (revision, leds, rumble) in [
        (1, [true, false, true, false], true),
        (2, [false, true, false, true], false),
    ] {
        indicators.apply_wii_outputs(gr_curated_controllers::wii_remote::WiiRemoteOutputs {
            revision,
            leds,
            rumble,
        });
        assert_eq!(indicators.wii_player_leds, Some(leds));
        assert_eq!(indicators.rumble_active, rumble);
        assert!(indicators.led.is_none());
        assert!(indicators.mute_led.is_none());
        let labels = render(|ui| {
            draw_controller_output_cards(
                ui,
                Kind::WiiRemote,
                RealizationId::LINUX_UHID_BLUETOOTH,
                &indicators,
            );
        });
        for label in [
            "Player 1",
            "Player 2",
            "Player 3",
            "Player 4",
            "Rumble",
            "Speaker (planned)",
            "State",
            "Format",
            "Volume",
            "Muted",
            "Audio plan phase A3",
        ] {
            assert!(
                labels.iter().any(|(text, _)| text == label),
                "missing {label}"
            );
        }
        assert_eq!(labels.iter().filter(|(text, _)| text == "On").count(), 2);
        assert_eq!(labels.iter().filter(|(text, _)| text == "Off").count(), 2);
        assert_eq!(labels.iter().filter(|(text, _)| text == "—").count(), 4);
        assert!(
            !labels
                .iter()
                .any(|(text, _)| text == "Lightbar" || text == "Mute")
        );
    }
    let labels = render(|ui| {
        draw_controller_output_cards(
            ui,
            Kind::WiiRemote,
            RealizationId::LINUX_UINPUT,
            &indicators,
        );
    });
    assert!(
        labels
            .iter()
            .any(|(text, _)| text == "Not exposed on Evdev / uinput")
    );
    assert!(labels.iter().any(|(text, _)| text == "Rumble"));
    assert!(!labels.iter().any(|(text, _)| text.starts_with("Player 1")));
    let other = render(|ui| {
        draw_controller_output_cards(
            ui,
            Kind::DualSense,
            RealizationId::LINUX_UHID_USB,
            &indicators,
        );
    });
    assert!(!other.iter().any(|(text, _)| text == "Speaker (planned)"));
}

#[test]
fn six_shapes_show_physical_button_groups_and_nunchuk_stick_beside_its_buttons() {
    for choice in [
        WiiExtensionChoice::None,
        WiiExtensionChoice::Nunchuk,
        WiiExtensionChoice::Classic,
    ] {
        for bridge in [false, true] {
            let ControllerView::WiiRemote(mut view) = editor::neutral_view(Kind::WiiRemote) else {
                unreachable!()
            };
            view.select_wii_attachment(choice, bridge);
            let groups = view.placed_button_groups();
            let native_ids: Vec<_> = view
                .current_topology()
                .auxiliary_buttons()
                .iter()
                .map(|button| button.id)
                .collect();
            for button in groups.iter().flat_map(|group| group.buttons) {
                if let Some(id) = button.id {
                    assert!(native_ids.contains(&id));
                }
            }
            let labels = render(|ui| {
                input_clusters::draw_editor(ui, 1, &mut view, &mut InputUiState::default());
            });
            assert!(labels.iter().any(|(text, _)| text == "Wii Remote buttons"));
            assert_eq!(
                labels.iter().any(|(text, _)| text == "Nunchuk buttons"),
                choice == WiiExtensionChoice::Nunchuk
            );
            assert_eq!(
                labels.iter().any(|(text, _)| text == "Classic buttons"),
                choice == WiiExtensionChoice::Classic
            );
            assert!(!labels.iter().any(|(text, _)| text == "Auxiliary buttons"));
            if choice == WiiExtensionChoice::Nunchuk {
                let buttons = labels
                    .iter()
                    .find(|(text, _)| text == "Nunchuk buttons")
                    .unwrap()
                    .1;
                let stick = labels
                    .iter()
                    .find(|(text, _)| text == "Nunchuk stick")
                    .unwrap()
                    .1;
                assert!((buttons.top() - stick.top()).abs() < 1.0);
                assert!(stick.left() > buttons.right());
            }
        }
    }
}
#[test]
fn remote_button_order_matches_the_controller_and_power_cannot_emit() {
    let ControllerView::WiiRemote(mut view) = editor::neutral_view(Kind::WiiRemote) else {
        unreachable!()
    };
    let mut state = InputUiState::default();
    let labels = render(|ui| input_clusters::draw_editor(ui, 1, &mut view, &mut state));
    let y = |name| {
        labels
            .iter()
            .find(|(text, _)| text == name)
            .unwrap()
            .1
            .center()
            .y
    };
    let sequence = [
        "Power",
        "Up",
        "Left",
        "Down",
        "A",
        "B (trigger)",
        "Home",
        "1",
        "2",
    ];
    assert!(sequence.windows(2).all(|pair| y(pair[0]) < y(pair[1])));
    let power = view.placed_button_groups()[0].buttons[0];
    assert_eq!(power.label, "Power");
    assert!(power.id.is_none());
    assert_eq!(power.help, Some("not an input report"));
    let context = egui::Context::default();
    let position = labels
        .iter()
        .find(|(text, _)| text == "Power")
        .unwrap()
        .1
        .center();
    for pressed in [true, false] {
        let _ = context.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    Pos2::ZERO,
                    Vec2::new(1500.0, 2400.0),
                )),
                events: vec![
                    egui::Event::PointerMoved(position),
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                ..Default::default()
            },
            |context| {
                egui::CentralPanel::default().show(context, |ui| {
                    input_clusters::draw_editor(ui, 1, &mut view, &mut state);
                });
            },
        );
    }
    assert!(view.take_edits().unwrap().is_empty());
}

#[test]
fn wii_motion_uses_declared_sensor_ranges_and_round_trips_native_values() {
    use gr_curated_controllers::wii_remote::WiiExtension;
    let ControllerView::WiiRemote(mut view) = editor::neutral_view(Kind::WiiRemote) else {
        unreachable!()
    };
    let remote = InputControlId::new("accel.x");
    let nunchuk = InputControlId::new("nunchuk.accel.x");
    let motion = view.motion_views()[0];
    assert!(!motion.gyroscope.unwrap().enabled);
    assert_eq!(motion.accelerometer.range.maximum, 1023);
    assert_eq!(motion.accelerometer.range.neutral, 0);
    assert_eq!(motion.gyroscope.unwrap().range.maximum, 16383);
    assert_eq!(motion.gyroscope.unwrap().range.neutral, 8192);
    view.apply_events(vec![InputEvent::Motion {
        id: remote,
        gyroscope: [1, 2, 3],
        accelerometer: [0, 511, 1023],
    }]);
    assert_eq!(
        view.motion_values(remote),
        Some(([8192; 3], [0, 511, 1023]))
    );
    let labels = render(|ui| {
        input_clusters::draw_editor(ui, 1, &mut view, &mut InputUiState::default());
    });
    assert!(labels.iter().any(|(text, _)| text == "needs MotionPlus"));
    assert!(labels.iter().any(|(text, _)| text == "Motion"));
    // Sensor axes are rendered only in Motion, never duplicate extra-axis cards.
    assert_eq!(
        labels.iter().filter(|(text, _)| text == "Accel X").count(),
        1
    );
    view.select_wii_attachment(WiiExtensionChoice::Nunchuk, true);
    assert!(view.motion_views()[0].gyroscope.unwrap().enabled);
    assert!(view.motion_views()[1].gyroscope.is_none());
    view.apply_events(vec![
        InputEvent::Motion {
            id: remote,
            gyroscope: [0, 8192, 16383],
            accelerometer: [1023, 1, 0],
        },
        InputEvent::Motion {
            id: nunchuk,
            gyroscope: [0; 3],
            accelerometer: [3, 700, 1023],
        },
    ]);
    assert_eq!(
        view.motion_values(remote),
        Some(([0, 8192, 16383], [1023, 1, 0]))
    );
    assert_eq!(view.motion_values(nunchuk), Some(([0; 3], [3, 700, 1023])));
    let WiiExtension::Nunchuk(n) = view.state().extension() else {
        unreachable!()
    };
    assert_eq!(n.accelerometer, [3, 700, 1023]);
    // Reject either invalid sensor atomically: no partial accelerometer update.
    for (gyro, accel) in [
        ([16384, 0, 0], [7; 3]),
        ([0; 3], [1024, 0, 0]),
        ([-1, 0, 0], [9; 3]),
    ] {
        let before = view.state().clone();
        assert!(!native_editor::apply_wii_event(
            &mut before.clone(),
            &InputEvent::Motion {
                id: remote,
                gyroscope: gyro,
                accelerometer: accel
            }
        ));
        view.apply_events(vec![InputEvent::Motion {
            id: remote,
            gyroscope: gyro,
            accelerometer: accel,
        }]);
        assert_eq!(view.state(), &before);
    }
    view.select_wii_attachment(WiiExtensionChoice::None, false);
    assert_eq!(view.motion_values(nunchuk), None);
    assert_eq!(view.motion_values(remote), Some(([8192; 3], [1023, 1, 0])));
}

#[test]
fn wii_ir_four_contacts_place_move_release_and_reject_invalid_points() {
    let ControllerView::WiiRemote(mut view) = editor::neutral_view(Kind::WiiRemote) else {
        unreachable!()
    };
    let ir = InputControlId::new("ir");
    let touch = view.touchpad_views()[0];
    assert_eq!((touch.width, touch.height, touch.contacts), (1024, 768, 4));
    assert!(matches!(
        touch.actuation,
        virtualgamepad::TouchpadActuation::None
    ));
    for contact in 0..4u8 {
        for point in [
            Some((0, 0)),
            Some((1023, 767)),
            Some((100 + u32::from(contact), 200)),
            None,
        ] {
            let before = view.touch_points(ir);
            view.apply_events(vec![InputEvent::Touch {
                id: ir,
                contact,
                point,
            }]);
            let after = view.touch_points(ir);
            assert_eq!(after[usize::from(contact)], point);
            for other in 0..4 {
                if other != usize::from(contact) {
                    assert_eq!(after[other], before[other]);
                }
            }
            assert_eq!(
                view.take_edits().unwrap().len(),
                1,
                "each accepted contact queues native set_ir"
            );
        }
    }
    for (contact, point) in [
        (0, Some((1024, 0))),
        (3, Some((0, 768))),
        (2, Some((u32::MAX, 0))),
        (4, Some((1, 2))),
        (u8::MAX, None),
    ] {
        let before = view.state().clone();
        view.apply_events(vec![InputEvent::Touch {
            id: ir,
            contact,
            point,
        }]);
        assert_eq!(view.state(), &before);
        assert!(view.take_edits().unwrap().is_empty());
    }
    let mut wrapped = ControllerView::WiiRemote(view);
    let failures = wrapped.take_diagnostics();
    assert_eq!(failures.len(), 5);
    assert!(failures.iter().all(|entry| !entry.success));
}

#[test]
fn wii_ir_renders_shared_touch_controls_without_actuation_or_legacy_list() {
    let ControllerView::WiiRemote(mut view) = editor::neutral_view(Kind::WiiRemote) else {
        unreachable!()
    };
    let context = egui::Context::default();
    let mut state = InputUiState::default();
    let mut labels = Vec::new();
    let mut pointer = Pos2::ZERO;
    for frame in 0..12 {
        let mut events = vec![egui::Event::PointerMoved(pointer)];
        if frame == 2 {
            events.push(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: Vec2::new(-1000.0, 0.0),
                modifiers: egui::Modifiers::default(),
            });
        }
        let output = context.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    Pos2::ZERO,
                    Vec2::new(1500.0, 2400.0),
                )),
                time: Some(f64::from(frame) * 0.1),
                events,
                ..Default::default()
            },
            |context| {
                egui::CentralPanel::default().show(context, |ui| {
                    view.draw_wii(ui, 12);
                    input_clusters::draw_editor(ui, 12, &mut view, &mut state);
                });
            },
        );
        let mut current = Vec::new();
        for shape in &output.shapes {
            text_shapes(&shape.shape, &mut current);
        }
        if frame == 1 {
            pointer = current
                .iter()
                .find(|(label, _)| label == "Contact 1")
                .unwrap()
                .1
                .center();
        }
        assert_eq!(
            current
                .iter()
                .filter(|(label, _)| label == "IR points")
                .count(),
            1
        );
        for item in current {
            if !labels.iter().any(|(label, _)| label == &item.0) {
                labels.push(item);
            }
        }
    }
    for expected in [
        "IR points",
        "Multitouch",
        "Contact 1",
        "Contact 2",
        "Contact 3",
        "Contact 4",
        "Reset touchpad",
    ] {
        assert!(
            labels.iter().any(|(label, _)| label == expected),
            "missing {expected}"
        );
    }
    assert_eq!(
        labels
            .iter()
            .filter(|(label, _)| label == "IR points")
            .count(),
        1
    );
    assert!(
        !labels
            .iter()
            .any(|(label, _)| label == "Deflection" || label.starts_with("Point "))
    );
}
