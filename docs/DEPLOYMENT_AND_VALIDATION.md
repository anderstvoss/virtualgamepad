# Deployment and validation

Current bounded alpha dispositions are recorded in the [alpha remediation status](history/alpha-1/ALPHA_REMEDIATION_STATUS.md).
Seven scoped installed broker security/recovery phases passed (#115 closed);
USB/IP audio remains opt-in WIP, with full routing and zero-loss qualification
deferred to #143/#145. The demo enables USB/IP audio and ALSA host routing by
default; add `--features audio-pipewire` for UHID/PipeWire audio. Library defaults
remain empty. Matching fidelity is unavailable (#116). Historical evidence below
retains its original scope.

Sustained audio continuity and latency are not validated in alpha; occasional audio dropouts may occur.

The [application API](APPLICATION_API.md) exposes uinput and USB/UHID for the
current controllers. Gadget sections below describe experimental research SPI,
not normal root creation or a supported alpha target. No automatic fallback or
runtime host preparation is introduced by the new application layer.


The realization IDs (`linux.uinput`, `linux.uhid.usb`, and `linux.dummy_hcd.usb-hid`) are peers. A controller is created only for the exact target selected by the application and declared by that controller. There is no target ordering and no fallback. The current dummy_hcd transport is unavailable: Linux f_hid does not expose complete GET_REPORT metadata or negative replies. Provider and broker creation reject it before resource construction; preparation cannot enable it.

`Evdev` uses an already accessible `/dev/uinput`. `Uhid` uses an already accessible `/dev/uhid`. Neither provider changes permissions or host setup.

## Privileged attachment targets

`DummyHcd` requires the root-owned `virtualgamepad-broker` service. Unprivileged clients use the fixed group-restricted Unix socket `/run/virtualgamepad/broker.sock`. The broker independently authenticates every client with `SO_PEERCRED` and accepts only administrator-authorized UIDs from `/etc/virtualgamepad/broker.conf`:

The socket directory is traversal-only (`root:root 0711`) so members of the
socket group can reach its fixed pathname without gaining directory listing
access. The socket remains `root:virtualgamepad 0660`; both that group check
and the broker's UID authorization must succeed.

```ini
# One entry per local application UID.
allow_uid=<application-uid>
instance=default
allow_udc=<reserved-dummy-udc>
```

Replace the placeholders with the intended numeric UID and an explicitly reserved
name such as `dummy_udc.0`. The config and every parent directory must be
root-owned, not group/world writable, and not symlinks. Older UID-only configs
now fail closed: add the instance and UDC authorization before upgrading.

Administrator setup loads `libcomposite`, `usb_f_hid` (unless built in), and
`dummy_hcd` as needed, and prepares the existing ConfigFS gadget mount. Runtime
creation never invokes modprobe or mounts filesystems. Install the optional
`systemd/virtualgamepad-broker.tmpfiles.conf` with the broker and apply it to
create its private state directories. For a non-default instance, the
administrator creates its own root-owned mode-0700 state directory.

Install `systemd/virtualgamepad-broker.socket` and `systemd/virtualgamepad-broker.service`, install the broker executable at `/usr/libexec/virtualgamepad/virtualgamepad-broker`, then enable the socket:

```bash
systemctl enable --now virtualgamepad-broker.socket
```

The service intentionally has no network listener. Its protocol only permits
opening one of the compiled `DualSense`, `DualShock4`, `SwitchPro`, or
standard-HID `Xbox360` profiles, their exact fixed-size input reports, bounded
reverse-output polling, close, and diagnostics. Session handles are
broker-created, connection-bound, and invalid after disconnect or restart.
Clients cannot supply USB descriptors, identities, module names, ConfigFS
paths, UDC choices, or report schemas.

The broker validates systemd's `LISTEN_PID`/`LISTEN_FDS` contract before it
performs stale ConfigFS recovery. It also holds an exclusive cross-process
lock and recovers only that instance's journaled gadgets whose filesystem
identity and authorized UDC binding still match. Unjournaled legacy gadgets
require explicit operator review; a name prefix is not proof of ownership.
Records survive service restarts under `/run/virtualgamepad-state` and are
removed only after successful cleanup. A crash between directory creation and
record creation leaves an unjournaled directory for operator review. A manually invoked broker refuses to replace
an existing socket and never cleans up service-owned gadgets.

## Demo validation

Current dummy_hcd acceptance is explicit rejection for all four families, with
the GET_REPORT limitation visible and no created resources. Use UHID/evdev for
available controller checks. The following prototype procedure is historical;
preparing the host no longer enables gadget creation.

For an interactive local check, run `cargo run -p virtualgamepad-demo`, select
one of the curated controllers and `USB / dummy_hcd`, then create it. The create panel
shows whether the broker socket is reachable; creation continues to provide the
authoritative authorization and host-preflight error. The DualSense, DualShock
4, and Switch Pro panels can exercise buttons, sticks, triggers, touch,
battery, and USB motion reports at 250 Hz. The Xbox 360 DummyHcd target is a
best-effort standard-HID USB attachment under the Xbox identity; it is not a
proprietary XInput/xpad implementation. Diagnostics and reverse-output
indicators show host activity.

## Host validation

The retained ignored gadget tests describe the earlier prototype, not current
successful construction. Do not blanket-enable them. Require the deterministic
rejection regressions and installed candidate rejection instead. Installed USB/IP
audio acceptance remains a separate gate; see [the remediation status](history/alpha-1/ALPHA_REMEDIATION_STATUS.md).

Root-only integration tests are intentionally ignored in normal test runs.
DummyHcd validation covers USB enumeration, HID feature exchange, motion input,
reverse output, and resource-only cleanup. The Linux `dummy_hcd` module exposes
a finite virtual-UDC resource: run controller-specific root tests against a
fresh attachment, rather than concurrently. SDL/Steam recognition and gyro
behavior remain controller-specific acceptance criteria. Bluetooth realizations require the separate L/M gates and have no supported installation path yet.

## Stateful UHID service and current migration boundary

Applications must call `service` on controller readiness and the next deadline even with unchanged semantic state. Required GET/SET requests are handled internally; user reply callbacks are not part of startup. A malformed request is rejected, and a transport whose delivery becomes uncertain is closed. Optional notifications can overflow only with an explicit dropped-event count.

The broker still uses its compiled startup feature path. Gate G must prove staged startup, control metadata/completion support, and latency before replacing it with unprivileged dynamic protocol handling. Do not infer gadget control-request parity from UHID tests. See [host prerequisites](history/alpha-1/HOST_READINESS.md) and the [reviewable provisioning proposal](history/alpha-1/HOST_PROVISIONING.md).

## Permission boundaries

Builds and deterministic tests need no device access or elevation. The Python
host inventory (`python3 scripts/host-preflight.py all`) only reads metadata and
configuration; it never opens device nodes or connects to the broker. Exit 1
means at least one prerequisite is missing, denied, mismatched, occupied, or
unvalidated. `available` is an observation, not authorization or reservation.
Run it for one complete realization ID to avoid unrelated prerequisites.

Direct creation access trusts all processes under the authorized identity to
create input devices, not only the curated gamepad profiles. Opt-in installations
may use the separate rules under `udev/` and matching groups
`virtualgamepad-uhid` and `virtualgamepad-uinput`; install only the selected
provider rule and explicitly enroll the intended user. Do not use world-writable
nodes or the broad `input` group. Existing distribution/session ACLs must be
reviewed rather than silently overwritten. The VM experiments use temporary
ACLs instead of persistent group enrollment.

For access lost after reboot, see the
[UHID research review and resolution plan](history/alpha-1/UHID_REBOOT_ACCESS_PLAN.md).
It separates registration, persistent authorization and temporary helper leases,
and defines the reboot evidence required before claiming a fix.

On hosts where UHID is modular and remains unregistered after boot, the device
node may exist with root-only permissions before the module's registration event
lets udev apply the installed policy. A GUI `cannot access device node` error
therefore requires checking `/sys/class/misc/uhid/dev` as well as the node ACL.
An administrator can load `uhid` and settle udev to test this case without an ACL
grant. If that alone restores creation, opt into boot registration using the
reviewed `modules-load.d/virtualgamepad-uhid.conf` file:

```bash
# From the repository root; inspect any existing destination before installing.
sudo install -o root -g root -m 0644 modules-load.d/virtualgamepad-uhid.conf /etc/modules-load.d/virtualgamepad-uhid.conf
```

Do not overwrite a different administrator-owned file. This configuration loads
only `uhid`; it grants no permissions and does not replace the explicit udev/group
policy. Verify ordinary-user creation after reboot without manual module loading.
To undo this opt-in, an administrator removes only this unchanged installed file;
do not unload the module while controllers may be using it.

For `linux.uinput` on kernels with `CONFIG_INPUT_UINPUT=m`, an administrator
can separately opt into boot registration using the
[`virtualgamepad-uinput.conf`](../modules-load.d/virtualgamepad-uinput.conf)
template. Inspect the destination before installing:

```bash
sudo install -o root -g root -m 0644 modules-load.d/virtualgamepad-uinput.conf /etc/modules-load.d/virtualgamepad-uinput.conf
```

This loads only `uinput`; it grants no permissions and does not replace the
explicit udev/group policy. Verify ordinary-user creation after reboot without
manual module loading. As with the UHID opt-in, remove only the unchanged
installed file to undo it, and do not unload a module in use. No entry is
needed when `CONFIG_INPUT_UINPUT=y`. Keep the UHID template scoped to `uhid`.

<a id="optional-sony-uhid-input-lab-isolation"></a>

### Optional Sony input-lab isolation

Active DualSense/DS4 touch tests can move the opted-in test input nodes onto an
inactive process-specific seat and set `LIBINPUT_IGNORE_DEVICE=1`. This policy
is limited to reviewed Sony UHID/uinput lab identities. For uinput, both the
gamepad and touch companion receive distinct process-owned lab labels; neither
is left on the normal desktop input path.

The helper runs as root from udev on the HID parent's `HID_PHYS` for UHID or
the inputN sysfs `ATTR{phys}` for uinput. It accepts only full-string ASCII lab
labels under `virtualgamepad/input-lab/{uhid,uinput}/{dualsense,dualshock4}/`,
at most 63 bytes,
with a canonical positive lowercase hexadecimal Linux PID (1 through
`0x7fffffff`) and bounded instance counter. The producer encodes these numeric
IDs in hexadecimal. Python isolated mode ignores user Python environment and
import paths. Helper logic performs no file/network access or shell invocation;
invalid labels or argument counts exit nonzero without output. Only successful
validation publishes the seat property used by input/event/hidraw rules.
Uinput validation uses `%s{phys}` from sysfs, not the quoted uevent `PHYS`.
Imports and the guards depending on them must remain separate, ordered rules.

For helper-validated uinput lab events only, an additional guard also checks
the uinput parent `ATTRS{phys}` and sets group `virtualgamepad-uinput`, mode
0660. The user must already belong to that dedicated creation group. Inactive
seats have no active-session ACL, so this narrow group grant enables SDL
read/write access while retaining the inactive seat and libinput-ignore gate.
It grants no physical-input access and does not change UHID event permissions.

After reviewing the helper and rules, inspect existing destination files and
install both artifacts from the repository root:

```bash
sudo install -o root -g root -m 0755 udev/virtualgamepad-input-lab-seat.py /usr/lib/udev/virtualgamepad-input-lab-seat.py
sudo install -o root -g root -m 0644 udev/71-virtualgamepad-input-lab.rules /etc/udev/rules.d/71-virtualgamepad-input-lab.rules
sudo udevadm control --reload
```

Both destination directories must be administrator-owned. Recreate the lab
device after reloading; do not use cached device numbers. The creating test
must explicitly set `VIRTUALGAMEPAD_INPUT_LAB=1`; normal demo/Steam creation
keeps its original identity. Keep Steam closed for active-contact SDL tests.
Before any active contacts, verify every owned event node's process-specific
`ID_SEAT` and `LIBINPUT_IGNORE_DEVICE=1`, and ordinary-user access to the exact
owned hidraw node for UHID, or every exact owned event node for uinput. The
ignored acceptance test enforces those gates. All live runs use
`--test-threads=1`; the expected seat is derived from the creating process ID,
with no separately supplied seat environment variable.

To undo the opt-in, first close the lab controllers, remove only these unchanged
installed helper/rule files, and reload udev. No general input-group enrollment
or blanket hidraw permission is needed. See
[SDL acceptance](SDL_ACCEPTANCE.md#active-sony-touch-acceptance-isolation) and the
[host inventory](plans/pre-controller-buildout/HOST_SETUP_HANDOFF.md#exact-udev-and-module-inventory).

Consumers need access to their session's hidraw/event nodes, not the creation
device or broker socket. SDL development files, Steam, capture tools, corpus
credentials, and physical reference devices are validation prerequisites only.
Audio/Bluetooth setup is deferred to its own realization gates.

The runtime service no longer has CAP_SYS_MODULE or ambient capabilities and
has ProtectKernelModules enabled. CAP_SYS_ADMIN is explicitly retained pending
the live Gate G reduced-capability test; its necessity is **not established**.
`systemd/virtualgamepad-broker-no-capabilities.conf` is an experimental drop-in,
not a validated default. Test it on the reserved UDC after administrator setup,
then remove CAP_SYS_ADMIN from the default only when that test passes. No
permission increase can restore missing gadget control metadata/completion.

An optional [local development helper](history/alpha-1/LOCAL_HOST_HELPER.md)
provides temporary creation-device ACLs, approved module loading, and configured
root-owned validation jobs. It is installed once by an administrator and is not a
product dependency or a general passwordless-sudo grant.

### Conventional evdev acceptance

`cargo test -p gr-curated-controllers --test evdev_feedback_live
all_families_complete_live_evdev_feedback -- --ignored --nocapture` (one shell
command) checks upload/update/play/stop/erase using exact created event nodes.
Run it as the ordinary user after selected uinput installation and consumer
access preparation. It needs neither SDL nor a privileged test suite. Full SDL
input parity remains separate. The demo observes typed playback activity; it no
longer acknowledges effect uploads or treats storing an effect as playback.


## Optional audio deployment boundary

For UHID/PipeWire, use the caller-session backend without privileged audio routing.
For USB/IP, follow the explicit administrator installer in the
[audio guide](CONTROLLER_AUDIO.md) and retain dedicated worker/allowlisted-port
ownership. Normal creation neither provisions modules nor changes permissions.
Seven scoped installed broker security/recovery phases passed (#115 closed).
Streaming remains opt-in WIP; this acceptance does not qualify continuity or the
full routing matrix. Never detach a remembered port without verified ownership.

The source-path root consumer check is `python3 scripts/check-root-consumers.py`.
It compiles the feature matrix with cached dependencies and then rebuilds offline;
it creates no controller. Exact-revision Git consumption remains a release gate.
Generate the current working inventory with strict all-feature root rustdoc and
`python3 scripts/generate-api-inventory.py`. Linux audio builds require their
existing PipeWire/SPA/ALSA headers and libclang; local build paths stay out of the repo.

## Xbox Series driver lab

This opt-in systemd helper lets one local account switch one attached wired
Xbox Series controller (`045e:0b12`) between stock `xpad` and the installed
`xone-wired` driver. An administrator runs the one-time install from a normal
host terminal in the repository:

```sh
sudo /usr/bin/python3 -I scripts/xbox-driver-lab.py install "$(id -un)"
```

Then the selected user can run these commands without sudo:

```sh
python3 scripts/xbox-driver-lab.py status
python3 scripts/xbox-driver-lab.py xone
python3 scripts/xbox-driver-lab.py xpad
```

Switching loads the needed modules and rebinds the selected USB interface.
Close controller and audio clients before switching and rediscover the nodes
afterward. `load-xone`, `load-xpad`, `unload-xone`, and `unload-xpad` are
available for explicit module tests. An unload leaves the reference unbound
if it used that driver and refuses if that driver owns another USB device.
The xone unload removes the wired transport module; its shared GIP modules
can stay loaded. The helper uses the installed modprobe policy.

For Bluetooth driver tests, `load-xpadneo` loads the installed
`hid_xpadneo` module and `unload-xpadneo` removes it. These operations
work with the USB controller disconnected. Removal permits one connected
Bluetooth Series `045e:0b13` HID device and refuses additional or other
HID devices bound to xpadneo. Module loading may select xpadneo on the
next Bluetooth reconnect. A controller already bound to Microsoft's HID
driver may need reconnecting to change its binding. USB xone/xpad switching
and Bluetooth xpadneo selection exercise their respective transports.

To update an existing trusted root-owned helper, an administrator runs:

```sh
sudo /usr/bin/python3 -I scripts/xbox-driver-lab.py install "$(id -un)" --replace
```

The update atomically replaces trusted installation files and keeps the
socket restricted to the selected account. The xpadneo module commands use
the existing module-loading capability. No additional sysfs write grant is
needed.


To revoke access, an administrator can run
`sudo systemctl disable --now virtualgamepad-xbox-driver-lab.socket`.

For development of the Xbox HIDAPI path, opt-in virtual Xbox nodes also
use the validated input-lab label. The updated lab-seat helper and 71 rule
grant access through the existing UHID group and active-seat hidraw tag.
The vendor xpadneo 60 rule can force a rebind during enumeration. Generate
a dev-only local wrapper with `scripts/prepare-xbox-bt-lab.py` and install
it as the local `60-xpadneo.rules` override. Its guard skips the vendor
rules only for the exact Bluetooth Xbox identity and helper-validated
virtual label. Physical-device rules are retained. Unload xpadneo before
a stock-driver run and record the actual bound driver after enumeration
settles. This remains opt-in test setup.

### Temporary Xbox Steam visibility

The normal input-lab policy isolates validated Xbox nodes on an inactive
seat and sets the input ignore flag. For owner-observed Steam validation
on seat0 only, the optional override below makes those nodes visible. It
does not affect physical pads or unrelated lab devices and is not installed
by default. Install before creating the manual probe:

```sh
sudo install -d -m 0755 /run/udev/rules.d
sudo install -o root -g root -m 0644 udev/90-virtualgamepad-xbox-steam.rules /run/udev/rules.d/90-virtualgamepad-xbox-steam.rules
sudo udevadm control --reload-rules
```

After closing and removing the virtual probe, restore normal isolation:

```sh
sudo rm /run/udev/rules.d/90-virtualgamepad-xbox-steam.rules
sudo udevadm control --reload-rules
```

Use `sweep-no-guide` for the Steam input test to avoid opening Big Picture.
Observe nonzero retained output and subsequent stops for Steam Ping; the
UI action alone does not establish reverse-command delivery. The completed
[Steam checkpoint](XBOX_SERIES_CHECKPOINT.md#owner-observed-steam-follow-up)
records the qualified versions, native units, owner observations and cleanup.
