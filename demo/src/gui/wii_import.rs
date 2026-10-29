//! Demo-only caller files. No values, presets or file-dialog dependencies.
use super::{RealizationId, egui};
use gr_curated_controllers::wii_remote::{WiiEepromImage, WiiExtensionCalibration};
use std::path::{Path, PathBuf};

const LABELS: [&str; 4] = [
    "EEPROM image",
    "MotionPlus block",
    "Nunchuk block",
    "Classic block",
];
const RESTRICTION: &str = "uinput has no EEPROM or extension calibration address channel";

#[derive(Default)]
pub(super) struct Imports {
    paths: [Option<PathBuf>; 4],
    picker: Option<Picker>,
}

impl Clone for Imports {
    fn clone(&self) -> Self {
        Self {
            paths: self.paths.clone(),
            picker: None,
        }
    }
}

#[derive(Clone)]
struct Entry {
    path: PathBuf,
    directory: bool,
}

#[derive(Clone, Default)]
struct Picker {
    slot: usize,
    directory: String,
    entries: Vec<Entry>,
    error: Option<String>,
}

pub(super) fn available(target: RealizationId) -> bool {
    target == RealizationId::LINUX_UHID_BLUETOOTH
}

pub(super) fn draw_compat(
    ui: &mut egui::Ui,
    target: RealizationId,
    revision: super::WiiRemoteRevision,
    options: &mut super::WiiRemoteCreationOptions,
) {
    ui.strong("MotionPlus compatibility");
    if !available(target) {
        ui.small("uinput has no addressed register channel for compatibility shims.");
    }
    ui.add_enabled_ui(available(target), |ui| {
        ui.checkbox(
            &mut options.motionplus_passthrough_id_mirror,
            "Passthrough ID mirror (compat)",
        );
        ui.small("Active child ID at F6; F7 stays 0x02, without challenge readiness.");
        ui.checkbox(
            &mut options.motionplus_passthrough_calibration_copy,
            "Child calibration copy (compat)",
        );
        ui.small("Exact imported child block at 0x40 during active passthrough.");
        ui.checkbox(
            &mut options.motionplus_activation_reattach_status,
            "Activation re-attach status (compat)",
        );
        ui.small("Send a status edge so consumers reselect reporting after MotionPlus activation.");
        if revision == super::WiiRemoteRevision::Tr {
            ui.checkbox(
                &mut options.motionplus_inactive_id_external_style,
                "External-style inactive MotionPlus ID (compat)",
            );
            ui.small("Use A6200005 instead of built-in A4200005 for Cemu activation.");
        }
    });
}

impl Imports {
    pub(super) fn load(&self, target: RealizationId) -> Result<WiiEepromImage, String> {
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
        for (slot, path) in self.paths.iter().enumerate() {
            if let Some(path) = path {
                bytes[slot] = Some(read(path).map_err(|e| format!("{}: {e}", LABELS[slot]))?);
            }
        }
        let image = WiiEepromImage::import(bytes[0].take().unwrap_or_default())
            .map_err(|e| format!("EEPROM image: {e}"))?;
        let mp = block::<32>(bytes[1].take(), LABELS[1])?;
        let n = block::<16>(bytes[2].take(), LABELS[2])?;
        let c = block::<16>(bytes[3].take(), LABELS[3])?;
        let calibration = WiiExtensionCalibration::import(mp, n, c)
            .map_err(|e| format!("Extension calibration: {e}"))?;
        Ok(image.with_extension_calibration(calibration))
    }

    pub(super) fn draw(&mut self, ui: &mut egui::Ui, target: RealizationId) {
        ui.strong("Caller calibration files");
        let enabled = available(target);
        if !enabled {
            self.picker = None;
            ui.label(RESTRICTION);
            ui.small("HID file choices are retained but not used for uinput.");
        }
        ui.add_enabled_ui(enabled, |ui| {
            for (slot, label) in LABELS.iter().enumerate() {
                ui.label(*label);
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
                    if ui.button("Choose file…").clicked() {
                        let directory = self.paths[slot]
                            .as_deref()
                            .and_then(Path::parent)
                            .map(Path::to_path_buf)
                            .or_else(|| std::env::current_dir().ok())
                            .unwrap_or_default();
                        let mut picker = Picker {
                            slot,
                            directory: directory.display().to_string(),
                            ..Picker::default()
                        };
                        picker.refresh();
                        self.picker = Some(picker);
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
        self.draw_picker(ui.ctx());
    }

    fn draw_picker(&mut self, ctx: &egui::Context) {
        if let Some(picker) = &mut self.picker {
            let mut open = true;
            let mut selection = None;
            egui::Window::new("Choose Wii calibration file")
                .id(egui::Id::new("wii-import-picker"))
                .default_width(480.0)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut picker.directory);
                        if ui.button("Go").clicked() {
                            picker.refresh();
                        }
                        if ui.button("Parent").clicked() {
                            if let Some(parent) = Path::new(&picker.directory).parent() {
                                picker.directory = parent.display().to_string();
                                picker.refresh();
                            }
                        }
                    });
                    if let Some(error) = &picker.error {
                        ui.colored_label(egui::Color32::LIGHT_RED, error);
                    }
                    let height = ui.text_style_height(&egui::TextStyle::Body) + 6.0;
                    egui::ScrollArea::vertical().max_height(280.0).show_rows(
                        ui,
                        height,
                        picker.entries.len(),
                        |ui, rows| {
                            for index in rows {
                                let entry = &picker.entries[index];
                                let name = entry
                                    .path
                                    .file_name()
                                    .unwrap_or(entry.path.as_os_str())
                                    .to_string_lossy();
                                let label = if entry.directory {
                                    format!("[Directory] {name}")
                                } else {
                                    name.into_owned()
                                };
                                if ui.selectable_label(false, label).clicked() {
                                    selection = Some(index);
                                }
                            }
                        },
                    );
                });
            if let Some(index) = selection {
                if let Some(path) = picker.select(index) {
                    self.paths[picker.slot] = Some(path);
                    open = false;
                }
            }
            if !open {
                self.picker = None;
            }
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

impl Picker {
    fn refresh(&mut self) {
        match std::fs::read_dir(&self.directory).and_then(|entries| {
            entries
                .map(|entry| {
                    entry.map(|e| Entry {
                        directory: e.path().is_dir(),
                        path: e.path(),
                    })
                })
                .collect::<Result<Vec<_>, _>>()
        }) {
            Ok(mut entries) => {
                entries.sort_by(|a, b| {
                    b.directory
                        .cmp(&a.directory)
                        .then_with(|| a.path.cmp(&b.path))
                });
                self.entries = entries;
                self.error = None;
            }
            Err(error) => {
                self.entries.clear();
                self.error = Some(error.to_string());
            }
        }
    }
    fn select(&mut self, index: usize) -> Option<PathBuf> {
        let entry = self.entries.get(index)?;
        if entry.directory {
            self.directory = entry.path.display().to_string();
            self.refresh();
            None
        } else {
            Some(entry.path.clone())
        }
    }
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
    fn valid_optional_blocks_are_preserved_and_creation_does_not_copy_a_picker() {
        let mut imports = Imports::default();
        imports.paths[1] = Some("mp.bin".into());
        imports.paths[2] = Some("n.bin".into());
        imports.paths[3] = Some("c.bin".into());
        imports.picker = Some(Picker::default());
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
            WiiExtensionCalibration::import(Some(mp), Some(n), Some(c)).unwrap(),
        );
        assert_eq!(image, expected);
        assert!(imports.clone().picker.is_none());
        assert_eq!(imports.clone().paths, imports.paths);
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
    fn switching_to_uinput_closes_the_picker_idempotently() {
        let mut imports = imports(1);
        imports.picker = Some(Picker::default());
        let ctx = egui::Context::default();
        for _ in 0..2 {
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    imports.draw(ui, RealizationId::LINUX_UINPUT);
                });
            });
            assert!(imports.picker.is_none());
            assert!(imports.paths[1].is_some());
        }
    }

    #[test]
    fn file_selection_uses_absolute_row_index_beyond_the_viewport() {
        let mut picker = Picker {
            entries: (0..130)
                .map(|n| Entry {
                    path: format!("fake-{n}.bin").into(),
                    directory: false,
                })
                .collect(),
            ..Picker::default()
        };
        for index in [0, 7, 64, 129] {
            assert_eq!(
                picker.select(index),
                Some(format!("fake-{index}.bin").into())
            );
        }
        assert_eq!(picker.select(130), None);
    }
}
