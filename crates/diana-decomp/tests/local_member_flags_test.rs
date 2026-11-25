mod support;

use diana_decomp::{
    input::{DexContainer, InputError, InputProblem, ProgramIndex},
    ByteOffset, FieldId, MethodId,
};

const DEX: &[u8] = include_bytes!("../../../quality/fixtures/atoms/field-basic/classes.dex");

fn program(bytes: &[u8]) -> ProgramIndex<'_> {
    DexContainer::open(bytes)
        .unwrap()
        .program(ByteOffset(0))
        .unwrap()
}

fn table_offset(header_field: usize) -> usize {
    u32::from_le_bytes(DEX[header_field..header_field + 4].try_into().unwrap()) as usize
}

#[test]
fn repeated_requests_keep_static_and_instance_member_flags() {
    let index = program(DEX);
    for _ in 0..2 {
        assert_eq!(index.local_field_flags(FieldId(2)).unwrap(), Some(0x02)); // value
        assert_eq!(index.local_field_flags(FieldId(0)).unwrap(), Some(0x0a)); // counter
        for (id, flags) in [(5, 0x09), (3, 0x01), (1, 0x10001)] {
            let method = index.local_method(MethodId(id)).unwrap().unwrap();
            assert_eq!(method.access_flags, flags);
            assert_eq!(method.owner_access_flags, 0x11);
            assert_eq!(method.parent.unwrap().bytes(), b"Ljava/lang/Object;");
            assert!(method.plain_scope);
        }
        assert!(index.local_method(MethodId(8)).unwrap().is_none()); // Object.<init>
    }
}

#[test]
fn malformed_field_sibling_keeps_valid_fields_and_its_own_error_on_retry() {
    let mut bytes = DEX.to_vec();
    // label's field_id.type_idx points to V. Its record boundaries and owner stay valid.
    let field_type = table_offset(0x54) + 8 + 2;
    bytes[field_type..field_type + 2].copy_from_slice(&4u16.to_le_bytes());
    let index = program(&bytes);
    let error = InputError::Malformed {
        code: InputProblem::InvalidDescriptor,
        offset_byte: ByteOffset(field_type),
    };
    for _ in 0..2 {
        assert_eq!(index.local_field_flags(FieldId(0)).unwrap(), Some(0x0a));
        assert_eq!(index.local_field_flags(FieldId(1)), Err(error));
        assert_eq!(index.local_field_flags(FieldId(2)).unwrap(), Some(0x02));
    }

    let engine = support::engine(&bytes);
    let owner = support::class(&bytes, &engine, "Lfixture/FieldBasic;");
    for _ in 0..2 {
        for (id, expected) in [(5, "return FieldBasic.counter;"), (3, "return this.value;")] {
            let result = engine.analyze_method(owner, MethodId(id)).unwrap();
            assert!(result.failures.is_empty());
            assert!(
                result.rendered.text().contains(expected),
                "{}",
                result.rendered.text()
            );
        }
        let failed = engine.analyze_method(owner, MethodId(4)).unwrap();
        assert_eq!(failed.failures.len(), 1);
        assert!(failed.failures[0].reason.contains(&error.to_string()));
    }
}

#[test]
fn method_lookup_reports_first_error_in_class_data_order_on_each_query() {
    let mut bytes = DEX.to_vec();
    // Direct readLabel (method #4) is visited before virtual addToInstance (#2).
    // Each method_id names Object instead of the owner that declares it.
    let methods = table_offset(0x5c);
    for id in [4, 2] {
        let owner = methods + 8 * id;
        bytes[owner..owner + 2].copy_from_slice(&2u16.to_le_bytes());
    }
    let bad_method = InputError::Malformed {
        code: InputProblem::InvalidReference,
        offset_byte: ByteOffset(methods + 8 * 4),
    };
    let index = program(&bytes);
    for id in [1, 3, 1] {
        assert_eq!(index.local_method(MethodId(id)).err(), Some(bad_method));
    }
    drop(index);

    // A damaged superclass is read only at the matched method. For <init>
    // its error precedes readLabel's, while for readInstance readLabel fails first.
    let parent = table_offset(0x64) + 8;
    bytes[parent..parent + 4].copy_from_slice(&0u32.to_le_bytes()); // I
    let bad_parent = InputError::Malformed {
        code: InputProblem::InvalidDescriptor,
        offset_byte: ByteOffset(table_offset(0x44)),
    };
    let index = program(&bytes);
    for (id, expected) in [(3, bad_method), (1, bad_parent), (3, bad_method)] {
        assert_eq!(index.local_method(MethodId(id)).err(), Some(expected));
    }
}
