# Alpha 2 P5a USB follow-up

Baseline virtualgamepad revision `6b6da124a1c28abcfb1a4595ede0da2e204351cd`. Steam and the stock UHID probes used this baseline. USB/IP timings list the exact local instrumentation revision used for each batch.

## USB/IP consumer timing

The installed broker supports three concurrent sessions. All listed batches used 50 cycles, `--sdl`, HIDAPI disabled for the SDL timing observer, and no budgets. DualSense, DualShock 4 and Xbox 360 declare this USB/IP realization. Switch Pro does not declare it.

Each measured removal checked the broker attachment directory against the file identity held at creation. Input and hidraw endpoints matched by canonical ancestry under that same attachment. The held file identity distinguishes the old attachment from a replacement on the same port.

| Family | N | Slots | API / present / consumer open / SDL / gone | Warm p50 ms in the same order | Revision |
| --- | ---: | ---: | --- | --- | --- |
| DualSense | 1 | 50 | 50 / 50 / 0 / 50 / 50 | 423.26 / 424.58 / — / 823.30 / 89.01 | `522de9054302eff82b9faea4c39d51e8fef8d10c` |
| DualSense | 3 | 150 | 150 / 150 / 0 / 150 / 150 | 409.02 / 411.49 / — / 920.44 / 179.55 | `522de9054302eff82b9faea4c39d51e8fef8d10c` |
| DualShock 4 | 1 | 50 | 50 / 50 / 50 / 50 / 50 | 423.11 / 424.46 / 456.73 / 807.86 / 65.32 | `a2c1c324d61bea4ddfcb1efcc917f1d876e3598a` |
| DualShock 4 | 3 | 150 | 150 / 150 / 150 / 150 / 150 | 404.44 / 406.57 / 438.56 / 879.10 / 151.63 | `a2c1c324d61bea4ddfcb1efcc917f1d876e3598a` |
| Xbox 360 | 1 | 50 | 50 / 50 / 50 / 50 / 50 | 378.40 / 379.98 / 499.80 / 827.49 / 32.13 | `a2c1c324d61bea4ddfcb1efcc917f1d876e3598a` |
| Xbox 360 | 3 | 150 | 150 / 150 / 150 / 150 / 150 | 360.27 / 363.82 / 481.42 / 847.69 / 120.09 | `a2c1c324d61bea4ddfcb1efcc917f1d876e3598a` |
| DualSense | 1 | 50 | 50 / 50 / 50 / 50 / 50 | 461.35 / 466.95 / 598.01 / 1758.12 / 149.54 | `a2c1c324d61bea4ddfcb1efcc917f1d876e3598a` |
| DualSense | 3 | 150 | 150 / 150 / 150 / 150 / 150 | 454.35 / 470.68 / 594.62 / 1991.83 / 213.83 | `a2c1c324d61bea4ddfcb1efcc917f1d876e3598a` |

The direct consumer open can run before its evdev ACL is available. The first DualSense revision measured that access denial even when SDL opened the same owned event afterward. The later instrumentation revision waits for the ACL on the selected event node. The three final N1/N3 pairs accounted for 600 successful creations, direct opens, SDL observations and measured removals. The USB/IP attach list was empty after the run.

## Stock Steam and SDL re-test

| Family | Bound driver | Steam controller log | Steam config match | SDL HIDAPI |
| --- | --- | --- | --- | --- |
| Switch Pro | nintendo | Fresh HIDAPI line present | 1 file(s) | Owned hidraw opened |
| DualSense | playstation | Fresh HIDAPI line present | 0 file(s) | Owned hidraw opened |

The config scan counted model matches. A matching file can predate this session. The Steam Settings window opened. The Controller page was unavailable to the capture both before and after restarting Steam. Steam client log recognition and an SDL HIDAPI open establish discovery below that UI. A candidate explanation for the earlier listing gap is filtering or registration after Steam HIDAPI recognition. The stock Switch Pro neutral and DualSense gyro movement remain unmeasured in this follow-up. The probes used the ordinary controller reports.

## Driver rebind

The earlier sudo journal showed `drivers/playstation/unbind`, `drivers/hid-generic/bind`, then `drivers/playstation/bind`. Together with the rollback branch in the P4 harness, this locates the reported failure at generic bind and indicates the stock driver needed an explicit restore. The journal did not retain the errno. Linux generic matching declines devices claimed by a special driver unless its ignore flag or device quirk allows the match. That is a candidate cause for this specific failure. [Linux generic matcher](https://github.com/torvalds/linux/blob/7b7fe306dd86b5342d3be8a3f1cbf3bb1ebf280f/drivers/hid/hid-generic.c)

This process has `NoNewPrivs` and cannot invoke sudo. The operator diagnostic records the exact operation, errno, and observed binding. It avoids the generic driver’s persistent dynamic-ID path on this kernel, which exposes `new_id` without `remove_id`. The generic comparison needs a privileged operator run.


| Family | Stock driver | Stock sessions | `driver_override` exposed |
| --- | --- | ---: | --- |
| dualsense | playstation | 3 | no |
| dualshock4 | playstation | 3 | no |
| switch-pro | nintendo | 3 | no |
| xbox360 | hid-generic | 3 | no |
