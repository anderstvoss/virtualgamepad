# Switch Pro: caller-supplied SPI read image (P10c)

This is a **source-supported, synthetic-by-default** controller-owned read image. No physical Switch Pro serial, address, calibration, firmware, or dump is included. Physical acceptance and host-specific calibration effects have not been measured.

## Memory model and import rules

`SwitchProSpiImage` accepts the factory SPI window `0x6000..0x6fff` and user window `0x8000..0x8fff` as caller-supplied raw regions. Inputs must lie completely within one window; empty extents, overflow, unmapped gaps and crossings are rejected without modifying the original image. Unprovided bytes return exactly the legacy `switch_spi_byte` synthetic defaults, including unchanged answers to all addresses outside the two windows.

Factory IMU and stick fields, factory RGB fields, and user calibration marker regions are defined by research claims [factory SPI map](https://github.com/anderstvoss/controller-protocol-research/blob/77b56f69631f1a0da39bb0e3e5ecbbf8e83329d8/records/claim/swpro-l0-device-factory-spi-map.json) and [user magic](https://github.com/anderstvoss/controller-protocol-research/blob/77b56f69631f1a0da39bb0e3e5ecbbf8e83329d8/records/claim/swpro-l0-device-user-spi-magic.json). Their evidence is source-only, not a measured retail image. [Linux fallback](https://github.com/anderstvoss/controller-protocol-research/blob/77b56f69631f1a0da39bb0e3e5ecbbf8e83329d8/records/claim/swpro-l0-consumer-kernel-calibration-fallback.json) and [SDL fallback](https://github.com/anderstvoss/controller-protocol-research/blob/77b56f69631f1a0da39bb0e3e5ecbbf8e83329d8/records/claim/swpro-l0-consumer-sdl-calibration-fallback.json) demonstrate distinct consumer decisions for absent user calibration and invalid factory fields. Do not invent per-device calibration.

## API, files, and lifecycle

Use `SwitchProSpiImage::import_raw(base, &bytes)`, optionally compose a second window using `with_region(base, &bytes)`, and pass the value with `SwitchProOptions::default().with_spi_image(image)` before creating a controller. Use `SwitchProController::spi_image()` to inspect/clone the retained image and recreate explicitly with the same image. Each session owns an independent copy.

`export_raw(base, len)` emits raw bytes **without an embedded base, family marker or header**. Persist the absolute SPI base alongside the file (the demo provides separate “Factory” and “User” buttons); each full window has 4096 bytes. The library never opens paths; the demo alone reads caller-selected input and performs atomic replacement outside Git checkouts. Do not commit imported files to a public repository.

Subcommand `0x10` reads the selected image. Device-info subcommand `0x02` advertises available SPI RGB only after all twelve factory RGB bytes have been explicitly supplied and at least one is not erased (0xff); otherwise its flag remains clear. This conservative source-only flag policy keeps an erased default export/import from becoming advertised colours; #161 owns the final colour semantics. The implementation does **not** accept host SPI writes `0x11` or `0x12` as mutations: it preserves its previous generic reply and does not mark the image dirty. Therefore the Wii EEPROM's host-write terminal snapshot/discard policy is not applicable; export is entirely caller-controlled.

The current Switch Pro implementations here are USB UHID and USB dummy-HCD; no Bluetooth realization is reported as landed. Tests are deterministic synthetic fixtures only. No physical Switch Pro test or host capture was run.
