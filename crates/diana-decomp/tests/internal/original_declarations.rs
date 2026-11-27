use super::{MemberKind, OriginalFlags, OriginalText};
use serde_json::json;

#[test]
fn flags_keep_raw_bits_and_member_specific_names_in_order() {
    let raw = 0x8000_00c1;
    for (kind, names, unknown) in [
        (MemberKind::Class, vec!["public"], 0x8000_00c0u32),
        (
            MemberKind::Field,
            vec!["public", "volatile", "transient"],
            0x8000_0000,
        ),
        (
            MemberKind::Method,
            vec!["public", "bridge", "varargs"],
            0x8000_0000,
        ),
    ] {
        let flags = OriginalFlags::new(raw, kind);
        assert_eq!(flags.raw(), raw);
        assert_eq!(
            serde_json::to_value(flags).unwrap(),
            json!({"raw": raw, "names": names, "unknown_bits": unknown})
        );
    }
}

#[test]
fn original_text_preserves_code_units_across_the_inline_boundary() {
    for n in [0, 1, 8, 9, 10, 11, 24] {
        let mut units = vec![u16::from(b'A'); n];
        if let Some(last) = units.last_mut() {
            *last = 0xd800;
        }
        let text = OriginalText::new(units.clone());
        assert_eq!(text.clone().units(), units);
        let expected = if n == 0 {
            String::new()
        } else {
            format!("{}\\uD800", "A".repeat(n - 1))
        };
        assert_eq!(
            serde_json::to_value(&text).unwrap(),
            json!({"text": expected, "units": units})
        );

        let returns = OriginalText::new(vec![u16::from(b'V')]);
        let proto = OriginalText::prototype(&[text], &returns).unwrap();
        let expected: Vec<_> = [u16::from(b'(')]
            .into_iter()
            .chain(units)
            .chain([u16::from(b')'), u16::from(b'V')])
            .collect();
        assert_eq!(proto.units(), expected);
    }
}
