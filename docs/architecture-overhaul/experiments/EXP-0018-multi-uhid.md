# EXP-0018 — Multiple exact UHID realizations (Gate Q)

Question: Can a shared controller-neutral UHID implementation serve complete USB
and Bluetooth realization IDs without fallback or controller branches?

Prerequisites: merged #106 baseline; existing fake UHID I/O and byte encoder.
Experiment: native HID specs carry their target; a configured provider validates
selection, native target, capability and bus before factory preflight/open.

Evidence: `exact_usb_and_bluetooth_selection_rejects_mismatch_before_any_io`
covers both valid paths, crossed providers, unknown IDs, crossed buses and native
selection disagreement. `create2_encodes_exact_controller_owned_metadata_for_both_buses`
checks the entire CREATE2 event. `mixed_uhid_buses_keep_independent_transport_labels`
checks fresh phys/uniq identities.
`concurrent_usb_and_bluetooth_sessions_close_independently` proves removing USB
does not stop Bluetooth servicing and each session destroys exactly once. Existing request/reply/cleanup tests are retained.

Result: deterministic target and byte-level tests pass. Existing controller
manifests remain USB-only and their descriptors/personality bytes are unchanged.
No live Bluetooth device or physical observation was performed.

Decision: accept complete target propagation and exact configured provider.
Consequences: struct-literal users supply `NativeHidRealization.target`; USB value
and default provider entrypoints remain available. Source aliases retain meaning.
Revisit condition: another compiled UHID path needs different metadata validation.
No new privileges. Q does not pass Bluetooth personality Gate L or radio Gate M.
