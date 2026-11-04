//! Faithful (fresh physical capture), not a synthesized compatibility descriptor.
//! Records: steam-s3-1302-report-shapes / steam-s3-1302-usb-interface.

pub(super) const VENDOR_ID: u16 = 0x28de;
pub(super) const PRODUCT_ID: u16 = 0x1302;
pub(super) const INTERFACES: u8 = 1;
pub(super) const WIRED_REPORT_DESCRIPTOR: &[u8; 372] =
    &include!("../../../../tests/fixtures/protocol-research/steam-s3-1302-report-descriptor.rs");

#[cfg(test)]
pub(super) mod tests;
