# DualSense LED output decoding — P2

Implementation notes for #154 and Alpha 2 #365.

This lane is source-evidence-only. Physical LED checks L1–L8 remain a separate local run and are not claimed here.

- USB report 0x02 LED fields must retain raw bytes and validity gating.
- Unknown mute LED values remain explicitly representable.
- Brightness encoding remains unresolved (`ds-cgap-led-brightness-encoding-disagreement`).
- Linux uinput has no native LED output path.
