# Brief 01: Switch Pro SPI fix (#174) and repo hygiene

Read `ground-rules.md` first.

**Lane:** Fixes. **Blocked by:** D1 (go-ahead for code). Nothing else.
**Runs alongside:** everything. Land 01a before brief 02 step 1a touches `switch_pro.rs`, to avoid a conflict.

## 01a. Switch Pro SPI reads (#174, parent #156)

Problem, verified on `3f34339` and still present on `5a0a7dd`: in `crates/gr-curated-controllers/src/switch_pro.rs`, `switch_spi_byte` (line 721) matches every address `>= 0x6020` through `checked_sub(0x6020)` and returns `IMU_CAL.get(offset).unwrap_or(0xff)`. So:

- the factory stick calibration branch at `0x603d` never runs, and
- the device-info reply (subcommand `0x02`) says colors are in SPI while `0x6050..0x605B` read back `0xFF` (an all-white controller in SDL `BReadDeviceInfo`).

Work:

1. Replace the open-ended match with explicit ranges: IMU calibration `0x6020..0x6038`, factory stick calibration `0x603d..0x604F`, colors `0x6050..0x605B`; everything else returns the documented erased value `0xFF`.
2. Colors: **default is to clear the "colors in SPI" flag** in the device-info reply, because no source-backed retail color set is recorded yet and the brief may not invent bytes. If a retail default with provenance exists in `controller-protocol-research` when you start, use it instead and cite the record ID. Configurable colors stay in #161.
3. Tests at the lowest seam (the subcommand `0x10` reply): each range, both boundaries of each, a read that straddles two ranges, a read wholly outside, and a test that the device-info flag agrees with what the color range returns.
4. Optional, if a host is available: an SDL probe check that SDL reads the expected stick calibration. Do not block the PR on it.

No public API change. Keep SPI interpretation inside the Switch Pro package; no generic storage abstraction (#162 owns that question).

Done when: the PR merges, #174 closes with a comment, #156 gets a one-line comment linking it.

## 01b. Hygiene (one small PR)

- Add `__pycache__/` and `*.pyc` to `.gitignore` (pulled forward from cleanup step 3, which then skips it).
- Bump the `protocol-research` submodule from `64196c5` to the research repo's current `main` (`259ff98` at time of writing, "Accept evdev trace fixtures from gamepadcapture (#6)"). Run `scripts/tests/test_protocol_research_pin.py` and whatever pin check the repo's docs name. If the bump needs a `.github/` change, stop and ask.

Done when: merged, and #176 has a one-line comment.
