# Compiled demo catalog and topology editors

The demo reads membership and audio capabilities from the supporting compiled
family registry. Its typed adapters preserve the existing selector order, labels,
native units, creation functions and worker lifecycle. This is an internal
workspace dependency on `gr-curated-controllers`, not a runtime profile or plugin
interface, and adds no third-party package or ordinary-root API item.

`gui.rs` coordinates generic creation, tabs, service workers and cleanup.
`families.rs` adapts compiled controller handles and family-specific output events;
`native_editor.rs` maps declared control IDs to typed native values/edits. One
renderer in `input_clusters.rs` draws the adapter's current `InputTopology`.
Adding a compiled family requires a typed native adapter, not a new GUI layout.
The exhaustive registry test rejects missing editor adapters.

The four fixed production families use their selected realization's surface.
The current-topology adapter seam also accepts a retained live subset; a synthetic
headless test swaps shapes and proves it does not cache the maximum. This does
not introduce a root live-topology getter or advertise a production attachment
family. D2/public API and controller-package decisions remain separate.

Before rendering, UI cache reconciliation retains declared control/cluster holds
and drops removed button, dpad and contact state. Reattachment starts released.
Native attachment transactions still own actual neutralization; UI state never
becomes a replacement controller state model. Unsupported native stick/motion/
axis adapters and custom modules display an explicit limitation instead of
silently disappearing. Existing typed event conversion, edit batching and
worker acknowledgement sequencing are preserved.

## Deterministic verification

- Every compiled registry family creates a neutral detached editor with the
  selected topology, native stick/trigger ranges, touch count and motion reader.
- Every registered editor renders headlessly without opening a provider.
- Current-shape changes add/remove the visible fixture control; removed holds and
  contacts do not revive after reattachment.
- A targeted regression first failed when the new reconciliation helper omitted
  motion IDs; it passes with complete declaration scanning. A still-declared
  motion hold survives, then remains released after removal/re-addition.
- Existing test modules in `gui.rs`, `editor.rs` and `input_clusters.rs` are
  byte-identical to their checkpoint-parent versions. Their lifecycle, focus,
  viewport, discovery isolation and terminal-cleanup regressions remain green.

No new live timing, physical input/output or sustained-audio acceptance is claimed.
