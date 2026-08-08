# Controller-family modeling guide

This guide records design boundaries for future curated packages. It is not a
claim that every listed controller is implemented.

| Controller | Native-state implication |
| --- | --- |
| Xbox 360 | Thumb-stick and trigger domains, guide/menu controls, conventional rumble, and controller-native headset/chatpad attachment semantics. |
| DualSense | Touch contacts, motion samples, lightbar, adaptive-trigger reports, conventional haptics, and audio/report controls remain native. Audio streams require an audio sidecar. |
| Switch 1 Joy-Con (L) | Left layout, IMU, HD rumble, rail/attachment topology. |
| Switch 1 Joy-Con (R) | Right layout plus NFC reader and IR Motion Camera as native protocols, not mouse/touch/axis aliases. |
| Switch 2 Joy-Con (L) | Left layout, motion, HD rumble, rail topology, and controller-native mouse sensing. |
| Switch 2 Joy-Con (R) | Right layout, motion, HD rumble, mouse sensing, and C/GameChat button. NFC and IR are evidence-required and must not be inferred from Switch 1. |
| Wii Remote | Motion/IR and expansion-port topology; Nunchuk, Classic Controller, and other extensions are controller-native attachment protocols. |
| Atari Jaguar | Distinct keypad and button topology; it must not be squeezed into a modern face-button/stick state. |

Controller families may share a value helper only after evidence establishes
the same unit and semantic meaning. They never share a required base state.
For every declared Linux target, the package documents the actual host surface
and rejects unavailable features rather than degrading them.
