//! Headless integration coverage for the Wii demo adapter and generic renderer.
use super::{
    editor::{self, ControllerView, WiiExtensionChoice},
    input_clusters::{self, InputUiState},
    native_editor::InputEditor,
    *,
};

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
