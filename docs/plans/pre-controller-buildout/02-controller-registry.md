# Brief 02: compiled controller registry (#173, cleanup step 1)

Read `ground-rules.md` first.

**Lane:** Core (serial). **Blocked by:** D1 (go-ahead for code).
**Gates:** every new controller family's wiring (#158, #159, #160), briefs 05, 06, 07, 09 and 10.

This is the critical path. It is the existing cleanup plan's step 1 (`plans/post-alpha-1-cleanup-agent-plan.md`, steps 1a to 1e), with three additions marked **new** so the registry does not need reshaping when Wii and Steam land.

## Why

Each family is hand-wired into closed enums and match arms outside its own module: `CompiledControllerKind` (`gr-realization-api`, used in `gr-provider-linux-dummy-hcd/src/lib.rs:68`), `ProfileId` (`gr-usbip/src/profile.rs:6`), the broker's per-family wire byte (`gr-privileged-broker/src/audio_client.rs`), `gr-audio-worker` string parsing and `run_typed` arms, `NativeState` (`gr-curated-controllers/src/usb_personality/state.rs:13`), the root facade and the demo. Three new families would add arms to every one.

## Steps (one PR each, in order; each leaves the four families working)

**1a. `ControllerFamily` trait and `FAMILIES` table, additive.** In `gr-curated-controllers`, extend `RealizationControllerDefinition` / `HidDriver` with:

- `const ID: ControllerId`
- `fn usb_profile() -> Option<UsbProfileSpec>` (device descriptor fields, HID report descriptor, playback and microphone channel layouts)
- `const BROKER_TAG: Option<u8>`, keeping DualSense 1, DualShock 4 2, Xbox 360 3
- `fn topology() -> &'static InputTopology`
- per declared realization: its **tier** (`PlayableReduced` for uinput, `PlayableFull` for UHID on the USB or Bluetooth bus, `FaithfulUsb` for USB/IP, `FaithfulRadio` for real Bluetooth) and **permission class** (`uinput`, `uhid`, `root-broker`)
- **new:** stub fields from the re-assessment, empty for the existing four: `max_topology` (defaults to `topology()`), `output_kinds`, `subsystems`, `persistent_resources`, `refused_operations`, and `kind: Controller | Receiver`. Keep them crate-internal (not re-exported from the root) so the API snapshot does not move. Briefs 05 to 08 fill them in.

Add `pub static FAMILIES: &[FamilyEntry]`. Tests: IDs unique, broker tags unique, every entry's declared realizations match its surfaces, every tier/permission pair is consistent (for example `PlayableFull` never needs `root-broker`).

**1b. `gr-usbip`: replace `ProfileId`.** `Profile::new` takes `&UsbProfileSpec` (or a registry key). `accepts_playback` / `accepts_microphone` read channel layouts from the spec, not `id == DualSenseEmulated`. First add a test that records the old descriptor bytes for all three profiles, then switch, then delete the enum. Bytes must be identical.

**1c. Broker and audio worker.** The broker client and server look the tag up in a table generated from `FAMILIES`, not a hand-written `match`. Wire format and protocol version unchanged. `gr-audio-worker` (`main.rs` parsing, `session.rs` `run_typed`, `client_pcm.rs`) dispatches through the registry. `NativeState` may stay an enum if removing it needs dynamic typing; say so in the PR. Existing broker security tests pass unchanged.

**1d.** Replace `CompiledControllerKind` with `ControllerId` lookups in `NativeDummyHcdRealization` and `dummy_hcd.rs`. (If D5 parks dummy_hcd before you get here, this step shrinks to deleting the enum's remaining uses.)

**1e. ADR-0018, broker allowlist.** The root broker serves only compiled profiles from the registry-generated table; adding a family is a reviewed broker release. Record the rejected alternative (a transport-only broker).

## Done when

- A test-only fifth family behind `cfg(test)`, never exported, needs only its module and one registry line.
- **new:** that test family declares `linux.uinput` **and `linux.uhid.bluetooth`**, and a fake-provider test creates it on the Bluetooth bus (bus `5`, as `gr-provider-linux-uhid/src/lib.rs:937` tests for the provider). This proves the Bluetooth-bus personality path Wii and Xbox Series need works through the curated session, not just the provider. If it does not, fix the gap in this step and say so in the PR.
- `docs/STATUS.md` and `docs/CORE_ARCHITECTURE.md` describe the registry.
- #173 closes; #176 lines 1a to 1e are ticked.

## Watch for

- Snapshot drift from doc comments on re-exported items.
- `gr-usbip` descriptor byte drift (the 1b comparison test catches it).
- Brief 01a edits `switch_pro.rs`; rebase onto it rather than reverting it.
