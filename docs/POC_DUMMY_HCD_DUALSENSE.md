# DualSense `dummy_hcd` USB-gadget POC

This experimental binary is isolated from the production UHID/provider library. It creates one
software USB DualSense through Linux ConfigFS and `dummy_hcd`, so Steam/SDL can be compared
against a physical USB DualSense without reusing a UHID topology.

Run it from the repository root:

```bash
cargo build -p poc-dualsense-dummy-hcd
sudo -E target/debug/poc-dualsense-dummy-hcd
```

Click **Start USB gadget**. The POC loads `libcomposite`, `usb_f_hid`, and `dummy_hcd` only when
needed, creates only `/sys/kernel/config/usb_gadget/virtualgamepad-poc-dualsense`, then binds its
single HID interface to the dummy UDC. Stop it with the GUI before exiting; cleanup unbinds and
removes only that POC gadget. A reported cleanup error means the gadget was deliberately retained
for inspection.

Identify the POC by its displayed `VG-POC-DS5-*` serial, not a `hidraw` number:

```bash
lsusb -d 054c:0ce6 -v | rg 'iSerial|bcdDevice'
udevadm info -q property -n /dev/hidrawN | rg 'ID_VENDOR_ID|ID_MODEL_ID|ID_SERIAL'
find /sys/bus/usb/devices -name serial -exec grep -H 'VG-POC-DS5-' {} +
```

The expected device is `054c:0ce6` with `bcdDevice 0110`. The POC log shows host feature probes
(`GET_REPORT 0x03`, `0x05`, `0x09`, `0x20`), HID OUT reports/rumble diagnostics, and a 250 Hz
motion-frame counter. It sends a neutral report immediately on the first 4 ms tick, then refreshes
motion continuously.

For Steam, restart Steam after starting the POC, then inspect its console log by serial:

```bash
rg 'VG-POC-DS5-|SDL_JOYSTICK_HIDAPI_PS5|controller device opened' \
  "$HOME/.steam/debian-installation/logs/console_log.txt"
```

Expected markers are an enabled `SDL_JOYSTICK_HIDAPI_PS5` driver, a non-zero USB release derived
from `bcdDevice`, controller open, and subsequent feature/motion polling. Compare those records to
a physical controller by serial and USB topology. Matching discovery plus Steam Controller Test
gyro mapping is evidence that USB topology, rather than UHID report encoding, was the missing
condition.
