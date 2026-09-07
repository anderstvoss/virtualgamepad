# Controller protocol and realization architecture

`virtualgamepad` is a standalone library for compiled, curated controllers. It does not own slot assignment, routing policy, runtime profiles, arbitrary identity selection, or automatic provider fallback.

## Complete realization IDs

Applications select one complete host-facing path declared by the controller manifest:

| ID | Mechanism | Implemented meaning |
| --- | --- | --- |
| `linux.uinput` | Linux uinput | Controller-owned evdev controls and supported outputs. |
| `linux.uhid.usb` | Linux UHID with USB bus metadata | Local HID protocol presentation; not an actual USB device. |
| `linux.dummy_hcd.usb-hid` | dummy_hcd and ConfigFS | Selectable broker-backed USB HID attachment. |

`RealizationId` is an extensible compiled string identifier. `RealizationTarget` and its `Evdev`, `Uhid`, and `DummyHcd` constants remain source aliases. `RealizationTargetSet::new(&[...])` declares a static membership set without a closed global enum. Unknown or mismatched paths fail preparation; they do not select another provider. Mechanism potential is not evidence of implemented or tested support.

## Ownership and data flow

```text
controller-native semantic state
    -> controller-owned stateful USB personality
    -> gr-hid runtime and logical HID commands
    -> Linux UHID transport and kernel envelopes
    -> host driver / application
```

DualSense, DualShock 4, Switch Pro, and the standard-HID Xbox 360 presentation use this path for UHID. Personalities own feature values, output acceptance, report sequences, timestamps, cadence, and handshake state. Shared snapshot helpers contain mechanics, with controller-supplied encoders and output validators. They do not select controller families.

The UHID provider handles CREATE2, INPUT2, output, GET/SET requests and replies, lifecycle notifications, readiness, and destruction. It contains no feature-response tables or controller-family branches. Required GET/SET processing is synchronous and independent of user callbacks. SET success follows personality validation. Unknown, malformed, and unsupported requests receive explicit errors or terminal cleanup if completion cannot be delivered safely.

Input/output/feature classes remain distinct. Logical payloads exclude a numbered report ID; serialization includes it once. Unnumbered reports use no logical ID. START flags are the runtime authority for numbering. STOP, START, consumer OPEN/CLOSE, and terminal library close have distinct meanings. Each request receives a session-scoped ordinal independently of a reusable kernel transaction ID.

The uinput path retains transactional evdev encoding and conventional force-feedback request handling. The existing dummy_hcd path retains its compiled broker startup behavior until Gate G establishes dynamic forwarding capability and latency. This is a migration boundary, not a claim that the broker rewrite is complete. The USB report encoders remain shared; a second mutable broker personality must not be introduced.

## State, scheduling, and delivery

Semantic edits clone and validate a candidate. Rejection preserves both accepted state and dirty status. HID timing belongs to the personality; changing motion state is not required to advance idle reports. DualSense and DS4 use the existing 4 ms compatibility cadence; Switch streams after its host handshake. These policies do not establish new physical timing evidence.

The runtime accepts an entire generated batch into a bounded queue before advancing protocol generation state. A definitely-unsent action retains its exact bytes. Partial success never replays earlier submissions; uncertain delivery terminates the session. Submission does not prove consumer observation or atomic visibility across components.

Each service call consumes at most one host event and makes a bounded number of submissions. Required replies have a reserved slot and a bounded retry deadline. Input receives service even under sustained requests. Optional output observations have separate bounded storage and an explicit loss counter; they cannot block required replies. Observations remain recoverable after a subsequent service error.

Call `poll_output` on controller readiness and at `next_service_in()`, even when input is unchanged. `commit()` also services HID work and preserves retryable accepted input when submission is blocked. `readiness()` exposes a borrowed descriptor where available. The caller owns the event loop and must stop using the descriptor after close. The demo continues to poll at its bounded USB cadence.

`close()` is terminal. Cleanup is attempted once, including failure; later edits and submissions fail. A host STOP cancels unsent input for that stopped presentation while preserving desired semantic state for START. Consumer CLOSE/OPEN is not terminal library close. Switch stream status and counters are read from the controller handle, not its semantic state snapshot.

## Feature parity, compounds, and audio

Each controller owns typed controls, numeric units, evdev presentation, outputs, and realization limitations. Motion, touch, LEDs, adaptive triggers, battery, and long-lived force-feedback objects must not silently disappear behind a generic gamepad model. Unsupported mappings must include a technical reason and regression coverage. Xbox's USB/HID presentation is explicitly not proprietary XInput/xpad emulation.

Existing compound helpers preserve preflight, reverse-order rollback, per-component identity, and cleanup regressions. Their older full-frame retry interface is not the new HID delivery contract; compound migration and advertised multi-UHID usefulness remain separately gated. Multiple UHID devices are not a composite USB device.

HID audio controls do not create PCM endpoints. Future controller-owned audio can use a coherent host-audio realization or actual USB audio functions through a suitable gadget realization. Neither route is implemented or validated merely by having audio contract types. Gates F/H/I control audio behavior and naming; Bluetooth personalities and actual Bluetooth bus realizations have separate L/M gates.

## Evidence and development

The independent private protocol corpus is pinned at `protocol-corpus/`. Records distinguish source support, synthetic fixtures, compatibility policy, physical observations, and conflicts. Checked-in test artifacts carry revision and input hashes. Ordinary Cargo builds require no corpus checkout, credentials, or network access to that repository.

Private fake-I/O seams and the deterministic protocol harness cover request sequences, exact replies, framing, delivery failures, lifecycle, and cleanup. Kernel, physical, SDL, Steam, audio, and Bluetooth acceptance are separate axes. See the [gate ledger](architecture-overhaul/GATE_STATUS.md), [deployment guide](DEPLOYMENT_AND_VALIDATION.md), and [supported-host procedure](SDL_ACCEPTANCE.md) for current limitations and reproducible validation.

Descriptor-based callers must watch readability and add writability while `wants_write()` is true, as well as servicing `next_service_in()`. This retries a blocked required reply before its terminal deadline. Polling callers may continue regular bounded service.
