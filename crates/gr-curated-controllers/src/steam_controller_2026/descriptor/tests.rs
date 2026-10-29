use super::*;
use std::collections::BTreeMap;

pub(in crate::steam_controller_2026) fn declared_reports() -> BTreeMap<(u8, u8), usize> {
    let mut globals = (0_u8, 0_usize, 0_usize); // report ID, size, count
    let mut stack = Vec::new();
    let mut bits = BTreeMap::<(u8, u8), usize>::new();
    let mut offset = 0;
    while offset < WIRED_REPORT_DESCRIPTOR.len() {
        let prefix = WIRED_REPORT_DESCRIPTOR[offset];
        offset += 1;
        assert_ne!(prefix, 0xfe, "captured descriptor has no long items");
        let size = [0, 1, 2, 4][usize::from(prefix & 3)];
        let mut value = 0_usize;
        for (index, byte) in WIRED_REPORT_DESCRIPTOR[offset..offset + size]
            .iter()
            .enumerate()
        {
            value |= usize::from(*byte) << (index * 8);
        }
        offset += size;
        let kind = (prefix >> 2) & 3;
        let tag = prefix >> 4;
        match (kind, tag) {
            (1, 7) => globals.1 = value,
            (1, 8) => globals.0 = u8::try_from(value).unwrap(),
            (1, 9) => globals.2 = value,
            (1, 10) => stack.push(globals),
            (1, 11) => globals = stack.pop().unwrap(),
            (0, 8 | 9 | 11) => *bits.entry((tag, globals.0)).or_default() += globals.1 * globals.2,
            _ => {}
        }
    }
    assert!(stack.is_empty());
    bits.into_iter()
        .map(|(key, bits)| (key, bits.div_ceil(8)))
        .collect()
}

#[test]
fn compiled_bytes_match_the_exact_imported_physical_fixture() {
    // Raw SHA-256 is independently enforced by the fixture generator and Python
    // regression. This equality binds that hashed binary to compiled bytes.
    let hex = include_str!(
        "../../../../../tests/fixtures/protocol-research/steam-s3-1302-report-descriptor.hex"
    );
    let expected = hex
        .split_whitespace()
        .map(|s| u8::from_str_radix(s, 16).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(WIRED_REPORT_DESCRIPTOR.as_slice(), expected);
    assert_eq!(WIRED_REPORT_DESCRIPTOR.len(), 372);
    assert_eq!((VENDOR_ID, PRODUCT_ID, INTERFACES), (0x28de, 0x1302, 1));
}

#[test]
fn captured_report_map_is_exact_including_unserved_declarations() {
    let expected = [
        (8, 0x40, 5),
        (8, 0x41, 8),
        (8, 0x42, 53),
        (8, 0x43, 14),
        (8, 0x44, 5),
        (8, 0x45, 45),
        (8, 0x79, 1),
        (8, 0x7b, 12),
        (9, 0x80, 9),
        (9, 0x81, 7),
        (9, 0x82, 3),
        (9, 0x83, 9),
        (9, 0x84, 8),
        (9, 0x85, 3),
        (9, 0x86, 3),
        (9, 0x87, 63),
        (9, 0x88, 63),
        (9, 0x89, 63),
        (11, 1, 63),
        (11, 2, 63),
    ]
    .into_iter()
    .map(|(kind, id, len)| ((kind, id), len))
    .collect::<BTreeMap<_, _>>();
    assert_eq!(declared_reports(), expected);
    assert!(!declared_reports().contains_key(&(8, 0x47)));
}
