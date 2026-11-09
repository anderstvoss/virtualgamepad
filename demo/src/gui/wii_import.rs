//! Demo-only caller files, selected with system dialogs. No factory presets.
use super::{RealizationId, egui};
use gr_curated_controllers::wii_remote::{WiiEepromImage, WiiExtensionCalibration};
use std::path::{Path, PathBuf};

const LABELS: [&str; 5] = [
    "Raw EEPROM bytes",
    "MotionPlus block",
    "Nunchuk block",
    "Classic block",
    "Saved EEPROM envelope",
];
const RESTRICTION: &str = "uinput has no EEPROM or extension calibration address channel";

#[derive(Clone, Default)]
pub(super) struct Imports {
    paths: [Option<PathBuf>; 5],
}

pub(super) fn available(target: RealizationId) -> bool {
    target == RealizationId::LINUX_UHID_BLUETOOTH
}

pub(super) fn draw_compat(
    ui: &mut egui::Ui,
    target: RealizationId,
    revision: super::WiiRemoteRevision,
    options: &mut super::WiiRemoteOptions,
) {
    use super::{Kind, families::draw_toggle};

    ui.strong("MotionPlus compatibility");
    if !available(target) {
        ui.small("uinput has no addressed register channel for compatibility shims.");
    }
    for toggle in Kind::WiiRemote.entry().creation_toggles {
        let (enabled, update): (_, fn(super::WiiRemoteOptions, bool) -> _) = match toggle.name {
            "motionplus_passthrough_id_mirror" => (
                options.motionplus_passthrough_id_mirror(),
                super::WiiRemoteOptions::with_motionplus_passthrough_id_mirror,
            ),
            "motionplus_passthrough_calibration_copy" => (
                options.motionplus_passthrough_calibration_copy(),
                super::WiiRemoteOptions::with_motionplus_passthrough_calibration_copy,
            ),
            "motionplus_activation_reattach_status" => (
                options.motionplus_activation_reattach_status(),
                super::WiiRemoteOptions::with_motionplus_activation_reattach_status,
            ),
            "motionplus_inactive_id_external_style" if revision == super::WiiRemoteRevision::Tr => {
                (
                    options.motionplus_inactive_id_external_style(),
                    super::WiiRemoteOptions::with_motionplus_inactive_id_external_style,
                )
            }
            _ => continue,
        };
        if let Some(value) = draw_toggle(ui, toggle, toggle.targets.contains(&target), enabled) {
            *options = update(std::mem::take(options), value);
        }
    }
}

impl Imports {
    #[cfg(test)]
    pub(super) fn envelope(path: PathBuf) -> Self {
        let mut imports = Self::default();
        imports.paths[4] = Some(path);
        imports
    }

    pub(super) fn load(
        &self,
        target: RealizationId,
        revision: super::WiiRemoteRevision,
    ) -> Result<WiiEepromImage, String> {
        if available(target) {
            if let Some(path) = &self.paths[4] {
                if self.paths[..4].iter().any(Option::is_some) {
                    return Err(
                        "Choose either a complete EEPROM envelope or separate raw imports".into(),
                    );
                }
                return super::wii_eeprom::read(path, revision);
            }
        }
        self.load_with(target, |path| {
            std::fs::read(path).map_err(|e| e.to_string())
        })
    }

    fn load_with(
        &self,
        target: RealizationId,
        mut read: impl FnMut(&Path) -> Result<Vec<u8>, String>,
    ) -> Result<WiiEepromImage, String> {
        // The disabled uinput options remain available when switching back to HID.
        if !available(target) {
            return Ok(WiiEepromImage::default());
        }
        let mut bytes: [Option<Vec<u8>>; 4] = std::array::from_fn(|_| None);
        for (slot, path) in self.paths.iter().take(4).enumerate() {
            if let Some(path) = path {
                bytes[slot] = Some(read(path).map_err(|e| format!("{}: {e}", LABELS[slot]))?);
            }
        }
        if bytes[0]
            .as_deref()
            .is_some_and(super::wii_eeprom::has_envelope_magic)
        {
            return Err("Saved EEPROM envelope selected as raw bytes; use Load EEPROM… so version, model and calibration are validated".into());
        }
        let image = WiiEepromImage::import(bytes[0].take().unwrap_or_default())
            .map_err(|e| format!("EEPROM image: {e}"))?;
        let mp = block::<32>(bytes[1].take(), LABELS[1])?;
        let n = block::<16>(bytes[2].take(), LABELS[2])?;
        let c = block::<16>(bytes[3].take(), LABELS[3])?;
        let calibration = WiiExtensionCalibration::default()
            .with_motion_plus(mp)
            .and_then(|calibration| calibration.with_nunchuk(n))
            .and_then(|calibration| calibration.with_classic(c))
            .map_err(|e| format!("Extension calibration: {e}"))?;
        Ok(image.with_extension_calibration(calibration))
    }

    pub(super) fn draw(&mut self, ui: &mut egui::Ui, target: RealizationId) {
        ui.strong("EEPROM and calibration imports");
        let enabled = available(target);
        if !enabled {
            ui.label(RESTRICTION);
            ui.small("HID file choices are retained but not used for uinput.");
        }
        ui.add_enabled_ui(enabled, |ui| {
            for slot in [4, 0, 1, 2, 3] {
                ui.label(LABELS[slot]);
                let text = self.paths[slot].as_ref().map_or_else(
                    || "None".into(),
                    |path| {
                        path.file_name()
                            .unwrap_or(path.as_os_str())
                            .to_string_lossy()
                    },
                );
                ui.label(text).on_hover_text(
                    self.paths[slot]
                        .as_ref()
                        .map_or_else(String::new, |p| p.display().to_string()),
                );
                ui.horizontal(|ui| {
                    let action = if slot == 4 {
                        "Load EEPROM…"
                    } else {
                        "Choose file…"
                    };
                    if ui.button(action).clicked() {
                        let mut dialog = rfd::FileDialog::new().set_title(LABELS[slot]);
                        if let Some(parent) = self.paths[slot].as_deref().and_then(Path::parent) {
                            dialog = dialog.set_directory(parent);
                        }
                        if slot == 4 {
                            dialog = dialog.add_filter("Wii EEPROM", &["wii-eeprom"]);
                        }
                        self.select(slot, dialog.pick_file(), target);
                    }
                    if ui
                        .add_enabled(self.paths[slot].is_some(), egui::Button::new("Clear"))
                        .clicked()
                    {
                        self.paths[slot] = None;
                    }
                });
            }
        });
        ui.small("Optional caller files only; no factory values or presets.");
    }

    fn select(&mut self, slot: usize, selection: Option<PathBuf>, target: RealizationId) {
        if !available(target) || slot >= self.paths.len() {
            return;
        }
        if let Some(path) = selection {
            if slot == 4 {
                self.paths[..4].fill(None);
            } else {
                self.paths[4] = None;
            }
            self.paths[slot] = Some(path);
        }
    }
}

fn block<const N: usize>(bytes: Option<Vec<u8>>, label: &str) -> Result<Option<[u8; N]>, String> {
    bytes
        .map(|bytes| {
            let size = bytes.len();
            bytes
                .try_into()
                .map_err(|_| format!("{label}: expected {N} bytes, got {size}"))
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn imports(slot: usize) -> Imports {
        let mut imports = Imports::default();
        imports.paths[slot] = Some(PathBuf::from("synthetic.bin"));
        imports
    }
    #[test]
    fn saved_envelopes_are_rejected_by_the_raw_import_path_without_nesting_or_model_bypass() {
        let bytes = super::super::wii_eeprom::encode(
            &WiiEepromImage::import(vec![0x37; 257]).unwrap(),
            super::super::WiiRemoteRevision::EarlyIntegratedPlus,
        )
        .unwrap();
        for length in [8, bytes.len()] {
            let error = imports(0)
                .load_with(RealizationId::LINUX_UHID_BLUETOOTH, |_| {
                    Ok(bytes[..length].to_vec())
                })
                .unwrap_err();
            assert!(error.contains("use Load EEPROM"));
        }
    }

    #[test]
    fn import_errors_are_reported_without_repair() {
        let target = RealizationId::LINUX_UHID_BLUETOOTH;
        assert!(
            imports(0)
                .load_with(target, |_| Ok(vec![0; 65537]))
                .unwrap_err()
                .contains("EEPROM")
        );
        for (slot, size) in [(1, 32), (2, 16), (3, 16)] {
            assert!(
                imports(slot)
                    .load_with(target, |_| Ok(vec![0; size - 1]))
                    .unwrap_err()
                    .contains("expected")
            );
            let error = imports(slot)
                .load_with(target, |_| Ok(vec![0; size]))
                .unwrap_err();
            assert!(error.contains(if slot == 1 { "CRC32" } else { "checksum" }));
        }
        assert!(
            imports(0)
                .load_with(target, |_| Err("synthetic read failure".into()))
                .unwrap_err()
                .contains("synthetic read failure")
        );
        assert_eq!(
            imports(0)
                .load_with(target, |_| Ok(vec![7; 42]))
                .unwrap()
                .bytes(),
            &[7; 42]
        );
    }
    #[test]
    fn valid_optional_blocks_and_recreation_imports_are_preserved() {
        let mut imports = Imports::default();
        imports.paths[1] = Some("mp.bin".into());
        imports.paths[2] = Some("n.bin".into());
        imports.paths[3] = Some("c.bin".into());
        let mut mp: [u8; 32] = std::array::from_fn(|i| u8::try_from(i).unwrap());
        // Synthetic CRC oracle; no captured or factory data.
        mp[14..16].copy_from_slice(&[0xb8, 0xa7]);
        mp[30..32].copy_from_slice(&[0x32, 0x5b]);
        let mut n = [1; 16];
        n[14..].copy_from_slice(&[0x63, 0xb8]);
        let mut c = [2; 16];
        c[14..].copy_from_slice(&[0x71, 0xc6]);
        let image = imports
            .load_with(RealizationId::LINUX_UHID_BLUETOOTH, |path| {
                Ok(match path.to_str().unwrap() {
                    "mp.bin" => mp.to_vec(),
                    "n.bin" => n.to_vec(),
                    "c.bin" => c.to_vec(),
                    _ => panic!("unexpected file"),
                })
            })
            .unwrap();
        let expected = WiiEepromImage::default().with_extension_calibration(
            WiiExtensionCalibration::default()
                .with_motion_plus(Some(mp))
                .and_then(|calibration| calibration.with_nunchuk(Some(n)))
                .and_then(|calibration| calibration.with_classic(Some(c)))
                .unwrap(),
        );
        assert_eq!(image, expected);
        assert_eq!(imports.clone().paths, imports.paths);
        let app = super::super::App::default();
        let saved = super::super::ControllerOptions {
            target: RealizationId::LINUX_UHID_BLUETOOTH,
            wii_imports: imports,
            ..app.creation_options()
        };
        let recreated = super::super::recreation_options(
            super::super::Kind::WiiRemote,
            &saved,
            app.creation_options(),
        );
        assert_eq!(recreated.wii_imports.paths, saved.wii_imports.paths);
    }

    #[test]
    fn uinput_gates_files_without_reading_or_erasing_hid_choices() {
        let imports = imports(1);
        assert!(!available(RealizationId::LINUX_UINPUT));
        assert!(
            imports
                .load_with(RealizationId::LINUX_UINPUT, |_| panic!(
                    "disabled file read"
                ))
                .unwrap()
                .bytes()
                .is_empty()
        );
        assert!(imports.paths[1].is_some());
    }
    #[test]
    fn switching_to_uinput_rejects_late_system_dialog_results_without_erasing_choices() {
        let mut imports = imports(1);
        for _ in 0..2 {
            imports.select(
                4,
                Some("ignored.wii-eeprom".into()),
                RealizationId::LINUX_UINPUT,
            );
            assert!(imports.paths[4].is_none());
            assert!(imports.paths[1].is_some());
        }
    }

    #[test]
    fn native_file_selection_uses_the_returned_path_and_cancellation_preserves_choices() {
        let target = RealizationId::LINUX_UHID_BLUETOOTH;
        let mut imports = imports(1);
        for index in [0, 7, 64, 129] {
            let path = PathBuf::from(format!("synthetic directory/fake-{index}.wii-eeprom"));
            imports.select(4, Some(path.clone()), target);
            assert_eq!(imports.paths[4], Some(path));
            assert!(imports.paths[..4].iter().all(Option::is_none));
            let before = imports.paths.clone();
            imports.select(0, None, target);
            assert_eq!(imports.paths, before);
        }
        imports.select(2, Some("synthetic-nunchuk.bin".into()), target);
        assert!(imports.paths[4].is_none());
        assert!(imports.paths[2].is_some());
        let before = imports.paths.clone();
        imports.select(130, Some("ignored.bin".into()), target);
        assert_eq!(imports.paths, before);
    }
}
