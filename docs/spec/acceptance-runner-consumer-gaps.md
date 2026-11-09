# Acceptance runner consumer gaps

[#166](https://github.com/anderstvoss/virtualgamepad/issues/166) has its
two-controller lifecycle baseline. The runner exercises exact registry targets,
concurrent ownership, peer error attribution and idempotent teardown. Pinned
[research #10](https://github.com/anderstvoss/controller-protocol-research/pull/10)
records 500 historical DualSense and Xbox 360 uinput samples. C5 adds 100 Wii
samples across uinput and Bluetooth-bus UHID. C6 adds 100 Steam Controller 2026
samples across uinput and wired UHID. Each result retains its original revision
and host. [C5 timing](../WII_REMOTE.md#creation-and-removal-timing) ·
[C6 timing](../C6_D9_ACCEPTANCE.md#timing-166)

The remaining integration is a creation-associated consumer oracle for the
first valid neutral input. The runner can time API return, matching node
discovery, consumer open where permitted, and disappearance. An accepted
commit or readable node does not identify when an application saw a decoded
neutral state. The SDL probe records completion as an upper bound. Permission
failures and missing SDL observations stay visible in their original runs.
Exercise the eventual oracle on the actual consumer and retain those outcomes
separately from the existing timing baselines. There is no universal latency
budget or physical timing claim.
