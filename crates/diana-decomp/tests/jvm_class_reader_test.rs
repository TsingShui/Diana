use diana_decomp::input::jvm::{
    ClassFile, ClassProblem, ClassScratch, Constant, JvmString, ReferenceKind,
};
use diana_decomp::FailureKind;

struct ClassFixture {
    bytes: Vec<u8>,
    field_name: usize,
    first_method_attr_length: usize,
}

fn u16_at(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend(value.to_be_bytes());
}

fn utf8(bytes: &mut Vec<u8>, value: &[u8]) {
    bytes.push(1);
    u16_at(bytes, u16::try_from(value.len()).unwrap());
    bytes.extend(value);
}

fn indirect(bytes: &mut Vec<u8>, tag: u8, index: u16) {
    bytes.push(tag);
    u16_at(bytes, index);
}

fn pair(bytes: &mut Vec<u8>, tag: u8, first: u16, second: u16) {
    indirect(bytes, tag, first);
    u16_at(bytes, second);
}

fn member(bytes: &mut Vec<u8>, flags: u16, name: u16, descriptor: u16, attributes: u16) {
    for word in [flags, name, descriptor, attributes] {
        u16_at(bytes, word);
    }
}

fn attribute(bytes: &mut Vec<u8>, name: u16, payload: &[u8]) {
    u16_at(bytes, name);
    bytes.extend(u32::try_from(payload.len()).unwrap().to_be_bytes());
    bytes.extend(payload);
}

fn fixture() -> ClassFixture {
    let mut bytes = vec![0xca, 0xfe, 0xba, 0xbe, 0, 0, 0, 61];
    u16_at(&mut bytes, 21);
    utf8(&mut bytes, b"demo/Example"); // 1
    indirect(&mut bytes, 7, 1); // 2
    utf8(&mut bytes, b"java/lang/Object"); // 3
    indirect(&mut bytes, 7, 3); // 4
    utf8(&mut bytes, b"java/io/Serializable"); // 5
    indirect(&mut bytes, 7, 5); // 6
    utf8(&mut bytes, b"count"); // 7
    utf8(&mut bytes, b"J"); // 8
    utf8(&mut bytes, b"compute"); // 9
    utf8(&mut bytes, b"(I)J"); // 10
    utf8(&mut bytes, b"Code"); // 11
    utf8(
        &mut bytes,
        &[
            b'A', 0xc0, 0x80, 0xed, 0xa0, 0xbd, 0xed, 0xb8, 0x80, 0xed, 0xa0, 0x80,
        ],
    ); // 12: A, NUL, 😀, isolated high surrogate
    indirect(&mut bytes, 8, 12); // 13
    bytes.push(5);
    bytes.extend((-25i64).to_be_bytes()); // 14 and the unusable 15
    pair(&mut bytes, 12, 9, 10); // 16: NameAndType
    pair(&mut bytes, 10, 2, 16); // 17: Methodref
    utf8(&mut bytes, b"BootstrapMethods"); // 18
    bytes.extend([15, 5, 0, 17]); // 19: REF_invokeVirtual
    utf8(&mut bytes, b"unavailable"); // 20
    for word in [0x0021, 2, 4, 1, 6] {
        u16_at(&mut bytes, word);
    }

    u16_at(&mut bytes, 2);
    let field_name = bytes.len() + 2;
    member(&mut bytes, 1, 7, 8, 0);
    member(&mut bytes, 2, 20, 8, 0);
    u16_at(&mut bytes, 2);
    member(&mut bytes, 1, 9, 10, 1);
    let first_method_attr_length = bytes.len() + 2;
    attribute(
        &mut bytes,
        11,
        &[0, 2, 0, 2, 0, 0, 0, 2, 0x09, 0xad, 0, 0, 0, 0],
    );
    member(&mut bytes, 0x0101, 20, 10, 0);
    u16_at(&mut bytes, 1);
    attribute(&mut bytes, 18, &[0, 1, 0, 19, 0, 1, 0, 13]);

    ClassFixture {
        bytes,
        field_name,
        first_method_attr_length,
    }
}

fn readable(name: JvmString<'_>) -> &str {
    name.utf8().unwrap()
}

#[test]
fn original_pool_members_and_attribute_offsets_are_available() {
    let fixture = fixture();
    let file = ClassFile::parse(&fixture.bytes).unwrap();
    assert_eq!(file.major_version(), 61);
    assert_eq!(file.minor_version(), 0);
    assert_eq!(file.access_flags(), 0x0021);
    assert_eq!(readable(file.name().unwrap()), "demo/Example");
    assert_eq!(
        readable(file.super_name().unwrap().unwrap()),
        "java/lang/Object"
    );
    assert_eq!(
        file.interfaces()
            .map(|name| readable(name.unwrap()))
            .collect::<Vec<_>>(),
        ["java/io/Serializable"]
    );
    assert_eq!(file.constant_pool_count(), 21);
    assert_eq!(file.constant(14).unwrap(), Constant::Long(-25));
    assert_eq!(
        file.constant(15).unwrap_err().problem,
        ClassProblem::InvalidConstantIndex(15)
    );
    assert_eq!(file.constant_tags().count(), 19);
    let (index, literal) = file.strings().next().unwrap().unwrap();
    assert_eq!(index, 13);
    assert_eq!(literal.to_utf16().unwrap(), [65, 0, 0xd83d, 0xde00, 0xd800]);
    assert!(literal.utf8().is_none());
    assert_eq!(literal.utf8_lossy().unwrap(), "A\0😀�");

    let fields: Vec<_> = file.fields().unwrap().map(Result::unwrap).collect();
    assert_eq!(
        fields
            .iter()
            .map(|field| (
                field.ordinal(),
                readable(field.name(&file).unwrap()),
                field.access_flags()
            ))
            .collect::<Vec<_>>(),
        [(0, "count", 1), (1, "unavailable", 2)]
    );
    assert_eq!(readable(fields[0].descriptor(&file).unwrap()), "J");
    let methods: Vec<_> = file.methods().unwrap().map(Result::unwrap).collect();
    assert_eq!(methods.len(), 2);
    assert_eq!(readable(methods[0].name(&file).unwrap()), "compute");
    assert_eq!(readable(methods[0].descriptor(&file).unwrap()), "(I)J");
    let code = file.code_attribute(methods[0]).unwrap().unwrap();
    assert_eq!(code.offset, fixture.first_method_attr_length + 4);
    assert_eq!(&code.payload[8..10], &[0x09, 0xad]);
    assert!(file.code_attribute(methods[1]).unwrap().is_none());

    let reference = file.member_reference(17).unwrap();
    assert_eq!(reference.kind, ReferenceKind::Method);
    assert_eq!(readable(reference.owner), "demo/Example");
    assert_eq!(readable(reference.name), "compute");
    assert_eq!(readable(reference.descriptor), "(I)J");
    let bootstrap = file.bootstrap_method(0).unwrap();
    assert_eq!((bootstrap.ordinal, bootstrap.method_ref), (0, 19));
    assert_eq!(bootstrap.arguments().collect::<Vec<_>>(), [13]);
}

#[test]
fn an_invalid_member_index_does_not_obscure_other_known_declarations() {
    let mut fixture = fixture();
    fixture.bytes[fixture.field_name..fixture.field_name + 2].copy_from_slice(&[0, 15]);
    let file = ClassFile::parse(&fixture.bytes).unwrap();
    let mut fields = file.fields().unwrap();
    let first = fields.next().unwrap().unwrap();
    let issue = first.name(&file).unwrap_err();
    assert_eq!(issue.offset, fixture.field_name);
    assert_eq!(issue.kind(), FailureKind::Malformed);
    assert_eq!(readable(first.descriptor(&file).unwrap()), "J");
    assert_eq!(
        readable(fields.next().unwrap().unwrap().name(&file).unwrap()),
        "unavailable"
    );
    assert_eq!(file.methods().unwrap().count(), 2);
}

#[test]
fn a_truncated_attribute_keeps_its_member_identity_and_bounds_the_failure() {
    let mut fixture = fixture();
    let at = fixture.first_method_attr_length;
    fixture.bytes[at..at + 4].copy_from_slice(&u32::MAX.to_be_bytes());
    let file = ClassFile::parse(&fixture.bytes).unwrap();
    let mut methods = file.methods().unwrap();
    let member = methods.next().unwrap().unwrap();
    assert_eq!(readable(member.name(&file).unwrap()), "compute");
    assert_eq!(readable(member.descriptor(&file).unwrap()), "(I)J");
    let issue = methods.next().unwrap().unwrap_err();
    assert!(methods.next().is_none());
    assert_eq!(issue.offset, at + 4);
    assert_eq!(issue.problem, ClassProblem::Truncated);
    assert_eq!(file.code_attribute(member).unwrap_err(), issue);
    assert_eq!(file.fields().unwrap().count(), 2);
}

#[test]
fn every_truncated_prefix_can_be_inspected_and_a_worker_can_reuse_its_storage() {
    let fixture = fixture();
    let mut scratch = ClassScratch::default();
    for length in 0..fixture.bytes.len() {
        let Ok(file) = ClassFile::parse_in(&fixture.bytes[..length], &mut scratch) else {
            continue;
        };
        assert_eq!(readable(file.name().unwrap()), "demo/Example");
        let _ = file.interfaces().collect::<Vec<_>>();
        let _ = file.fields().map(|fields| fields.collect::<Vec<_>>());
        if let Ok(methods) = file.methods() {
            for method in methods.flatten() {
                let _ = file.code_attribute(method);
            }
        }
        let _ = file.bootstrap_method(0);
    }
    let file = ClassFile::parse_in(&fixture.bytes, &mut scratch).unwrap();
    assert_eq!(file.bootstrap_method(0).unwrap().method_ref, 19);
}

#[test]
fn malformed_literal_encoding_stays_local_across_ascii_word_boundaries() {
    let mut fixture = fixture();
    let file = ClassFile::parse(&fixture.bytes).unwrap();
    let literal = file.utf8(12).unwrap();
    let start = literal.offset();
    let length = literal.bytes().len();
    fixture.bytes[start..start + length].fill(b'x');
    for at in start..start + length {
        for invalid in [0, 0x80, 0xff] {
            fixture.bytes[at] = invalid;
            let file = ClassFile::parse(&fixture.bytes).unwrap();
            let issue = file.strings().next().unwrap().unwrap_err();
            assert_eq!(issue.offset, at);
            assert_eq!(issue.problem, ClassProblem::InvalidModifiedUtf8);
            assert_eq!(readable(file.name().unwrap()), "demo/Example");
            assert_eq!(file.methods().unwrap().count(), 2);
        }
        fixture.bytes[at] = b'x';
    }
}
