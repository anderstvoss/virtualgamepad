//! Demo-only caller-selected Switch Pro SPI image files. No device data is bundled.
use super::{egui, wii_eeprom};
use virtualgamepad::SwitchProSpiImage;

const WINDOW_SIZE: usize = 0x1000;

#[derive(Clone, Default)]
pub(super) struct Settings {
    image: SwitchProSpiImage,
    message: Option<String>,
}

impl Settings {
    pub(super) fn image(&self) -> &SwitchProSpiImage {
        &self.image
    }

    fn load_raw(&mut self, base: u32, bytes: &[u8]) -> Result<(), String> {
        let next = self.image.clone().with_region(base, bytes).map_err(|e| e.to_string())?;
        self.image = next;
        Ok(())
    }

    fn save_raw(&self, base: u32, path: &std::path::Path) -> Result<(), String> {
        let bytes = self.image.export_raw(base, WINDOW_SIZE).map_err(|e| e.to_string())?;
        wii_eeprom::validate_destination(path)?;
        wii_eeprom::write(path, &bytes)
    }

    pub(super) fn draw(&mut self, ui: &mut egui::Ui) {
        ui.strong("Switch Pro SPI image");
        ui.small("Raw files: factory base 0x6000; user base 0x8000. Each window is 4096 bytes. Only caller-selected files are read or written.");
        for (label, base) in [("Factory", 0x6000_u32), ("User", 0x8000)] {
            ui.horizontal(|ui| {
                if ui.button(format!("Load {label}…")).clicked() {
                    if let Some(path) = rfd::FileDialog::new().set_title(format!("Load {label} SPI bytes")).pick_file() {
                        self.message = Some(match std::fs::read(path) {
                            Ok(bytes) => match self.load_raw(base, &bytes) {
                                Ok(()) => format!("{label} SPI bytes loaded"),
                                Err(error) => format!("{label} SPI import: {error}"),
                            },
                            Err(error) => format!("{label} SPI read: {error}"),
                        });
                    }
                }
                if ui.button(format!("Save {label}…")).clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title(format!("Save {label} SPI window"))
                        .set_file_name(format!("switch-pro-{}.bin", label.to_lowercase()))
                        .save_file()
                    {
                        self.message = Some(match self.save_raw(base, &path) {
                            Ok(()) => format!("{label} SPI bytes exported"),
                            Err(error) => format!("{label} SPI export: {error}"),
                        });
                    }
                }
            });
        }
        if let Some(message) = &self.message {
            ui.small(message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reject_bad_import_without_changing_current_image() {
        let mut settings = Settings::default();
        settings.load_raw(0x8010, &[0x71, 0x72]).unwrap();
        let before = settings.image().clone();
        assert!(settings.load_raw(0x8fff, &[0x73, 0x74]).is_err());
        assert_eq!(settings.image(), &before);
    }
}
