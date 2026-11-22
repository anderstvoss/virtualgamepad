# Alpha 2 controller polish cleanup

Baseline virtualgamepad revision `d9cc4445bcd0901f84d10635c6b82403f0e1d765`. The virtual host tests used SDL 3.4.20.
The research records link the baseline experiments to the existing claims. [Research PR #167](https://github.com/anderstvoss/controller-protocol-research/pull/167).

## USB stock and generic binding

| Family | Stock driver | Stock closes | Privileged generic result |
| --- | --- | --- | --- |
| DualSense | playstation | 3 | ENODEV in 3 attempts |
| DualShock 4 | playstation | 3 | ENODEV in 3 attempts |
| Switch Pro | nintendo | 3 | ENODEV in 3 attempts |
| Xbox 360 | hid-generic | 3 | Already generic in all three sessions |

Three sessions per family used its stock Linux driver. All 12 exposed owned evdev and hidraw nodes, opened through SDL evdev, and removed the owned HID parent on close. SDL HIDAPI opened three sessions each for DualSense, DualShock 4, and Switch Pro, and zero for Xbox 360. The privileged comparison tried only the exact owned HID parent. The harness restored the stock driver on each `ENODEV`. Xbox 360 starts with `hid-generic`, so its three sessions also cover the generic condition.

## Steam Controller settings

Steam listed the stock USB-bus Switch Pro and displayed neutral sticks and triggers in three tester frames. Its generic arm was unavailable after the bind failures. Steam listed USB-bus DualSense and opened Gyro Calibration. Neutral, single-axis gyro, and return-to-neutral commits were acknowledged. All six frames during the gyro stimulus matched the neutral frame. The Steam gyro check remains open. The owned windows closed and their devices were removed.

## Wii Bluetooth consumers

The stock Wii driver bound, and its `uniq` matched the typed synthetic identity. Four of four synthetic EEPROM chunks matched through an exact-node hidraw ACL. After an `ENOMEM` probe and memory compaction, SDL default enumeration found one owned evdev joystick. SDL did not list the owned hidraw through its default HIDAPI configuration. The original ACL was verified before close, and the device was removed.

The retail-descriptor PR was still open at review. P6 default enumeration, HIDAPI claim and Dolphin reruns remain pending on the merged revision. [Retail descriptor PR](https://github.com/anderstvoss/virtualgamepad/pull/432).

## Synthetic calibration

The same motion stimulus produced the predicted shift in the kernel and SDL HIDAPI for DualSense USB and Bluetooth and DualShock 4 USB. A separate stress arm doubled the Sony sensor span. The kernel followed that change, and SDL stayed at its fallback value. The passing arm used an independent bounded calibration within SDL's accepted range. [SDL DualSense calibration code](https://github.com/libsdl-org/SDL/blob/release-3.4.2/src/joystick/hidapi/SDL_hidapi_ps5.c#L582-L625).

The same left-stick stimulus produced the predicted SPI calibration shift for Switch Pro on USB and Bluetooth through hid-nintendo, SDL evdev, and SDL HIDAPI. DualShock 4 Bluetooth returned the expected refusal for caller-supplied data.

## Bluetooth-bus UHID lifecycle and output

| Family | Exact evdev mappings | Exact SDL evdev mappings | Typed outputs | Separate closes |
| --- | --- | --- | --- | --- |
| DualSense | 8/8 | 1/8 | 2/2 | 2 |
| DualShock 4 | 8/8 | 1/8 | 2/2 | 2 |
| Switch Pro | 6/8 | 4/8 | 2/2 | 2 |

Both virtual instances in each pair bound to their expected stock driver and appeared in evdev and SDL. The stimulated unit's input moved in both consumers. The peer stayed neutral through the sequence, survived the first close, and closed independently. Separate timestamped rumble and LED actions matched typed output on the stimulated unit. Switch Pro used its typed HOME light. Exact SDL mapping varied by backend. The recorded counts above use SDL evdev and the exact single-control oracle.

Each timing batch ran 50 cycles with `--sdl` and no latency budgets. The baseline runner recorded the first available owned nodes before a joystick was classified. Its SDL stage reported no owned node for the requested backend. The direct-open stage also saw early access denials and missing nodes.

| Family | Concurrency | Slots | Created | Present | Direct open | SDL open | Removed |
| --- | --- | --- | --- | --- | --- | --- | --- |
| DualSense | 1 | 50 | 50 | 50 | 9 | 0 | 50 |
| DualSense | 3 | 150 | 150 | 150 | 10 | 0 | 150 |
| DualShock 4 | 1 | 50 | 50 | 50 | 1 | 0 | 50 |
| DualShock 4 | 3 | 150 | 150 | 150 | 8 | 0 | 150 |
| Switch Pro | 1 | 50 | 50 | 50 | 2 | 0 | 50 |
| Switch Pro | 3 | 150 | 150 | 150 | 2 | 0 | 150 |

## Authorized private checks

Two Original Wii EEPROM Save/Load cycles matched the private source envelope. Both virtual instances were removed. The USB DualSense read-only capture returned both requested feature reports. On each virtual DualSense bus, both reports matched after accounting for Bluetooth CRC framing, and kernel and SDL calibration and firmware matched. Both virtual realizations were removed.
