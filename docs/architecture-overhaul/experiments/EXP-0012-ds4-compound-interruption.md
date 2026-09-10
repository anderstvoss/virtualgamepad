# EXP-0012 — DS4 split evdev prototype and display interruption

## Result and acceptance boundary

The test-only DS4 prototype separates a conventional gamepad from a two-contact
absolute touch companion. One ten-second SDL 3.2.0 run on Linux 6.12.107 recorded
exact gamepad selection, neutral/buttons/axes, typed conventional rumble, consumer
reaping and removal of both nodes. The user then reported that the display
session crashed. The intended three-run experiment did not finish. This is an
interrupted experiment, not a DS4 acceptance pass or proof of crash causation.

The subsequent inventory found no surviving acceptance process or DS4 test node.
No shared display service was restarted and no permissions, modules or persistent
host settings were changed. Available user error logs did not establish a cause;
the coredump inspection utility was unavailable. Raw output stays outside Git.

## Retained implementation and deterministic evidence

The prototype is compiled only under `cfg(test)` and uses injected providers.
Production DS4 creation and the existing live harness remain unchanged. Its
combined-node SDL discovery restriction remains in place.

Controller-owned code splits capabilities and full snapshots, retains both MT
slots/tracking IDs/coordinates/releases, and derives legacy coordinates from the
first active contact. The gamepad owns FF requests and replies; the companion
has no FF capability. The native SDL mapping revision is 0x8111, matching the
pinned database's native layout, not a physical firmware claim.

Fake-provider tests cover split capabilities, contact activation/release,
second-open rollback, partial snapshot retry, exact feedback routing, repeated
polling, activity during arbitrary removal with reused application IDs, terminal
shutdown and idempotent reverse-order cleanup. These protect library behavior;
they do not reproduce or explain a display-server crash.

## Prerequisites for another live run

1. Inspect available display/session failure evidence without restarting shared
   services or changing global device policy. Preserve uncertainty if unavailable.
2. Use a separate VM clone or an isolated consumer session where the contact node
   cannot reach the user's active desktop. Headless SDL alone is insufficient:
   another desktop consumer can still open the global input device.
3. Identify both run-owned nodes, record their classification and consumers, and
   validate touch slots and releases through the exact evdev interface before
   scripted activity. Do not infer SDL touch association from gamepad discovery.
4. Run three repetitions plus consumer-exit/controller-removal tests, verify both
   nodes disappear and all children are reaped, then review production adoption.

No extra standing privileges resolve this consumer-isolation requirement. Other
family/core tests and Gate G work remain independent.
