# Controller-native core specification

## Product boundary

`virtualgamepad` is a compiled, curated-controller emulator. It is not a
runtime profile engine, YAML controller-definition system, or plugin host.
The current migration deliberately exposes no production controller
constructors: controller implementations will be restored as reviewed,
first-party packages after the core is complete.

Steam Controller 1 is not an active target. Its pre-deactivation source is
preserved on the `archive/steam-controller-1` branch; it must not be restored
or advertised by this migration.

## Independent realization modes

The controller model and the way a host sees that model are separate. Every
controller has one typed state and one normalized/native command vocabulary.
The selected realization mode controls only the host presentation.

- `HostCompatible` is a generic Linux-host presentation, normally uinput.
- `IdentityAccurate` is a local software HID identity with reviewed
  descriptors/reports, normally UHID. It is not a USB or Bluetooth device-role
  claim.
- `HardwareFaithful` is device-role transport/topology behavior, normally a
  USB gadget realization.

These modes are independent, not ordered tiers. A controller may implement any
non-empty subset. Creation always selects one exact `LinuxTarget`; the target
declares one mode. Unsupported targets fail before a handle exists and never
fall back to another target or mode.

The same controller calls have the same semantic meaning in every supported
mode. A feature that cannot faithfully be exposed in the selected mode returns
`ControlError::UnavailableInRealizationMode` and preserves state. A feature
the controller family never has returns `UnsupportedControl` instead.

## Core boundaries

`gr-realization-api` owns controller-neutral target, mode, prepared OS
realization, raw-frame, backend-session, and provider-capability contracts.
`gr-controller-contract` owns normalized controls, controller definitions,
realization manifests, and lifecycle/control errors. `gr-controller-runtime`
owns atomic state updates, dirty/retry semantics, and mode-aware validation.
Linux providers own only operating-system I/O and prepared-realization shape
validation. None may branch on a controller family.

A future controller package supplies its typed state, native controls,
capabilities, manifest entries, codecs, reverse-event decoder, and one root
creation function. It must not require runtime or provider controller branches.
Its concrete handle exposes typed feature capabilities; a heterogeneous handle
never accepts stringly typed native features.

## Acceptance and testing

Every manifest entry names its exact target(s), prerequisites, host-visible
feature surface, fidelity claim, and reverse-output availability. Contract,
runtime, provider, property/state-machine, compile-fail, and hermetic
integration tests must prove exact selection, no fallback, mode-gated atomic
failure, retry after send failure, callback isolation, and clean shutdown.

YAML is allowed only for sanitized fixtures and snapshots. It never defines
runtime controller behavior.
