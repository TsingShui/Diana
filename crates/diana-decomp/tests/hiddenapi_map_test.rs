mod support;

use diana_decomp::{ByteOffset, DexImage, Engine, ImageOrigin, InputId, Request};

const DEX: &[u8] =
    include_bytes!("../../../quality/fixtures/atoms/static-field-values/classes.dex");

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}

fn set_u32(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn with_hiddenapi() -> (Vec<u8>, usize) {
    let mut bytes = DEX.to_vec();
    let old_map = u32_at(&bytes, 0x34) as usize;
    let count = u32_at(&bytes, old_map) as usize;
    let mut records = bytes[old_map + 4..old_map + 4 + count * 12].to_vec();
    bytes.resize(bytes.len().next_multiple_of(4), 0);
    let hidden = bytes.len();
    let hidden_size = 4 + u32_at(&bytes, 0x60) as usize * 4;
    bytes.resize(hidden + hidden_size, 0);
    set_u32(&mut bytes, hidden, hidden_size as u32);
    let map = bytes.len();
    for record in records.as_chunks_mut::<12>().0 {
        if u16::from_le_bytes(record[..2].try_into().unwrap()) == 0x1000 {
            record[8..12].copy_from_slice(&(map as u32).to_le_bytes());
        }
    }
    records.extend_from_slice(&0xF000u16.to_le_bytes());
    records.extend_from_slice(&0u16.to_le_bytes());
    records.extend_from_slice(&1u32.to_le_bytes());
    records.extend_from_slice(&(hidden as u32).to_le_bytes());
    let mut records = records
        .as_chunks::<12>()
        .0
        .iter()
        .map(|record| record.to_vec())
        .collect::<Vec<_>>();
    records.sort_by_key(|record| u32_at(record, 8));
    bytes.extend_from_slice(&(records.len() as u32).to_le_bytes());
    let mut hidden_record = 0;
    for record in records {
        if u16::from_le_bytes(record[..2].try_into().unwrap()) == 0xF000 {
            hidden_record = bytes.len();
        }
        bytes.extend_from_slice(&record);
    }
    let len = bytes.len() as u32;
    let data_start = u32_at(&bytes, 0x6c);
    set_u32(&mut bytes, 0x20, len);
    set_u32(&mut bytes, 0x34, map as u32);
    set_u32(&mut bytes, 0x68, len - data_start);
    (bytes, hidden_record)
}

fn admitted(bytes: &[u8]) -> bool {
    Engine::open(vec![DexImage {
        bytes,
        header_offset: ByteOffset(0),
        origin: ImageOrigin { input: InputId(0) },
    }])
    .is_ok()
}

#[test]
fn opaque_hiddenapi_metadata_keeps_original_class_readable() {
    let (bytes, _) = with_hiddenapi();
    let engine = support::engine(&bytes);
    let class = support::class(&bytes, &engine, "Lfixture/StaticFieldValues;");
    let analysis = engine
        .analyze(Request {
            class,
            method: None,
        })
        .expect("original class");
    assert!(analysis.failures.is_empty());
    assert!(analysis.rendered.text().contains("int CLOSE = 5;"));
}

#[test]
fn hiddenapi_map_envelope_still_rejects_bad_offsets_and_counts() {
    let (bytes, record) = with_hiddenapi();
    for (field, value) in [(8, u32::MAX), (8, u32_at(&bytes, record + 8) + 1), (4, 0)] {
        let mut invalid = bytes.clone();
        set_u32(&mut invalid, record + field, value);
        assert!(
            !admitted(&invalid),
            "invalid map field {field} value {value}"
        );
    }
    let mut duplicate = bytes.clone();
    duplicate[record + 12..record + 14].copy_from_slice(&0xF000u16.to_le_bytes());
    assert!(!admitted(&duplicate), "duplicate hidden-API map entry");
}
