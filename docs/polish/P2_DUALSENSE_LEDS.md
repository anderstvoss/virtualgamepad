# DualSense LED output decoding — P2

Implementation notes for #154 and Alpha 2 #365.

This lane is source-evidence-only. Physical LED checks L1–L8 remain a separate local run and are not claimed here.

- USB report 0x02 LED fields must retain raw bytes and validity gating.
- Unknown mute LED values remain explicitly representable.
- Brightness encoding remains unresolved (`ds-cgap-led-brightness-encoding-disagreement`).
- Linux uinput has no native LED output path.

## Implemented source-only interpretation

- `DualSenseMuteLed` preserves off/on/pulse and any unrecognized byte. Research: `ds-cgap-mute-led-pulse-source-mapping`.
- `player_leds` contains only indicator bits 0–4. `player_led_instant` represents bit 5 (source-backed by `ds-cgap-player-led-instant-source-mapping`).
- `led_brightness` is the raw byte. Source implementations conflict on numerical levels and gating: `ds-cgap-led-brightness-encoding-disagreement`. No physical-level mapping is claimed.
- `lightbar_setup` is a validity-gated raw setup byte; `release_leds` represents an asserted flag-only command rather than a persistent state.
- The retained `DualSenseOutputState.player_leds` remains the unmasked player byte for backwards compatibility, unlike the new parsed event's five-bit indicator field.
- Linux uinput has no native LED output delivery, and `feature_available` is not a host output-subscription API.
- Deterministic decoder and GUI-state tests exercise valid gating and preserved bytes. Proposed L1–L8 hardware checks: **not run**.

Research basis: [DualSense LED inventory](https://github.com/anderstvoss/controller-protocol-research/blob/77b56f69631f1a0da39bb0e3e5ecbbf8e83329d8/docs/dualsense-led-inventory.md).
