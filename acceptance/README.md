# Per-function acceptance manifests

One TOML file per compiled production family, named by its full ControllerId.
Each native-function row has a cell for every declared realization. These cells
track acceptance, separately from implementation. No cell is promoted here.

Schema version 1 requires controller, the 40-character pinned research revision,
realizations, refused_operations, persistent_resources and functions. Each cell
has one of `not-supported`, `wip`, `source-evidence`, `physical-evidence`, a logical
acceptance test ID, research record IDs in evidence and a reason. Test IDs reserve
acceptance cases; they do not assert that a live run or test already exists.
Accepted source/physical cells require real record IDs at research_revision;
empty evidence is honest for pending/refused cells and requires a reason.
No repository path is evidence. A virtual-device run is not physical evidence.

The initial rows cover controls, outputs, identity, accessories and lifecycle
from CONTROLLER_SUPPORT.md and its linked historical function review. All usable
paths remain wip. The dummy_hcd cells record the existing f_hid contract refusal.
Refusals/resources are empty until typed registry declarations supply them;
these files are acceptance data, never runtime profiles or device configuration.

`cargo test -p gr-curated-controllers --test acceptance_manifests` checks actual
TOML against the compiled production registry, including complete cell coverage.
