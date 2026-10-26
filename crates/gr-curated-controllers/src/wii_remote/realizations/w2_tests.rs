//! Independent HID short-item walk; no candidate retail descriptor bytes.
use super::DESCRIPTOR;
use std::collections::BTreeMap;

#[test]
fn interim_gamepad_collection_preserves_vendor_variable_report_table() {
    // W2 option B, #158 comment6354044408; wii-c5-report-layouts.
    let mut cursor = 0;
    let mut page = 0;
    let mut usage = None;
    let mut report_id = 0;
    let mut size = 0;
    let mut count = 0;
    let mut depth = 0;
    let mut collections = Vec::new();
    let mut reports = BTreeMap::new();
    while cursor < DESCRIPTOR.len() {
        let prefix = DESCRIPTOR[cursor];
        assert_ne!(prefix, 0xfe, "no long items in the authored descriptor");
        let length = match prefix & 3 {
            3 => 4,
            n => usize::from(n),
        };
        let payload = &DESCRIPTOR[cursor + 1..cursor + 1 + length];
        let value = payload.iter().enumerate().fold(0_u32, |value, (i, byte)| {
            value | (u32::from(*byte) << (8 * i))
        });
        cursor += 1 + length;
        let kind = (prefix >> 2) & 3;
        let tag = prefix >> 4;
        match (kind, tag) {
            (1, 0) => page = value,
            (1, 7) => size = value,
            (1, 8) => report_id = value,
            (1, 9) => count = value,
            (2, 0) => usage = Some(value),
            (0, 10) => {
                if depth == 0 {
                    collections.push((page, usage.unwrap(), value));
                }
                depth += 1;
            }
            (0, 12) => {
                assert!(depth > 0);
                depth -= 1;
            }
            (0, direction @ (8 | 9 | 11)) => {
                assert_eq!(depth, 1);
                assert_eq!(page, 0xff00, "report fields stay vendor-defined");
                assert_eq!(value, 2, "Data/Variable/Absolute flags retained");
                assert_eq!(usage, Some(1));
                assert_ne!(report_id, 0);
                *reports.entry((direction, report_id)).or_insert(0) += size * count;
            }
            _ => {}
        }
        if kind == 0 {
            usage = None;
        }
    }
    assert_eq!(depth, 0);
    assert_eq!(
        collections,
        [(1, 5, 1)],
        "one Game Pad Application collection"
    );
    let expected = [
        (8, 0x20, 6),
        (8, 0x21, 21),
        (8, 0x22, 4),
        (8, 0x30, 2),
        (8, 0x31, 5),
        (8, 0x32, 10),
        (8, 0x33, 17),
        (8, 0x34, 21),
        (8, 0x35, 21),
        (8, 0x36, 21),
        (8, 0x37, 21),
        (8, 0x3d, 21),
        (8, 0x3e, 21),
        (8, 0x3f, 21),
        (9, 0x10, 1),
        (9, 0x11, 1),
        (9, 0x12, 2),
        (9, 0x13, 1),
        (9, 0x14, 1),
        (9, 0x15, 1),
        (9, 0x16, 21),
        (9, 0x17, 6),
        (9, 0x18, 21),
        (9, 0x19, 1),
        (9, 0x1a, 1),
    ]
    .into_iter()
    .map(|(direction, id, bytes)| ((direction, id), bytes * 8))
    .collect();
    assert_eq!(
        reports, expected,
        "exact directions/IDs/payload bits, including W8 control and rumble-only data handling; no features"
    );
}
