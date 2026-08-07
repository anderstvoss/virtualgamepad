# Fuzz targets

These `cargo-fuzz` targets exercise caller-driven control sequences and raw
reverse reports. They are intentionally outside the normal workspace so stable
builds do not require nightly Rust or `libFuzzer`.

```bash
cargo install cargo-fuzz
cargo +nightly fuzz run reverse_reports
cargo +nightly fuzz run control_sequences
```

Commit only minimized, synthetic regression inputs. Never add captures that
contain user identifiers, host paths, device serials, or private traffic.
