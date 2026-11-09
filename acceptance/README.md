# Per-function acceptance manifests

One TOML file per compiled production family, named by its full ControllerId.
Each native-function row has a cell for every declared realization. These cells
track evidence for acceptance separately from implementation.

Schema version 1 requires controller, the 40-character pinned research revision,
realizations, refused_operations, persistent_resources and functions. Each cell
has one of `not-supported`, `wip`, `source-evidence`, `virtual-live`, or
`physical-evidence`. Each cell also includes a logical acceptance test ID,
research record IDs in `evidence` and a reason. Test IDs reserve
acceptance cases; they do not assert that a live run or test already exists.
Accepted source/physical cells require real record IDs at research_revision;
empty evidence is honest for pending/refused cells and requires a reason.
For `virtual-live`, `live_evidence` links a checked-in `docs/*.md` report
(optionally with a section anchor) and the reason states what the real host
observed on the owned virtual device, including any disagreement or failure.
The `evidence` array remains for pinned research record IDs. A virtual-live
result does not imply physical validation, full acceptance or a fresh run.

The initial rows cover controls, outputs, identity, accessories and lifecycle
from CONTROLLER_SUPPORT.md and its linked historical function review. The
dummy_hcd cells record the existing f_hid contract refusal.
Refusals/resources are empty until typed registry declarations supply them;
these files are acceptance data, never runtime profiles or device configuration.

`cargo test -p gr-curated-controllers --test acceptance_manifests` checks actual
TOML against the compiled production registry, including complete cell coverage.

The pinned protocol-research check also resolves each manifest's evidence IDs at
its declared research_revision, which must be an ancestor of the adopted corpus
pin. A record present only in a newer checkout cannot satisfy an older declaration;
older declarations remain valid when they contain their referenced evidence.

The timing runner includes `evidence_status: "virtual-live"` when a Linux run
observes a creation-owned virtual node. Fake runs and runs without an observed
node emit null. Older v1 run records can omit the field. The status describes
run provenance; consult sample errors and missing stages for individual results.
The runner does not rewrite acceptance manifests.
