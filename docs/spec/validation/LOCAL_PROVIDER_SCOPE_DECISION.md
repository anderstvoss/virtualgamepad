# Decision: Linux targets are explicit

Status: accepted

Every creation call selects exactly one `LinuxTarget`: `Uinput`, `Uhid`, or
`UsbTransport`. There is no provider inventory, planner, scope default, or
automatic fallback in the public API.

`Uinput` requires `/dev/uinput`; `Uhid` requires `/dev/uhid`; USB transport
requires configfs gadget support, a peripheral-capable UDC, and an observing
host. The controller's compiled realization is validated before open. Missing
Cargo features, incompatible controller/target combinations, and host open
failures remain distinct actionable creation errors.

The demo must present these choices and prerequisites before creation. A
future provider target requires an explicit public contract and security model;
it cannot be inserted as an implicit fallback.
