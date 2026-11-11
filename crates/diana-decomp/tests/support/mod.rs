use diana_decomp::{
    input::DexContainer, ByteOffset, ClassKey, DexId, DexImage, Engine, ImageOrigin, InputId,
};

pub fn engine(bytes: &[u8]) -> Engine<'_> {
    Engine::open(vec![DexImage {
        bytes,
        header_offset: ByteOffset(0),
        origin: ImageOrigin { input: InputId(0) },
    }])
    .expect("fixture DEX")
}

pub fn class(bytes: &[u8], engine: &Engine<'_>, descriptor: &str) -> ClassKey {
    let container = DexContainer::open(bytes).expect("fixture envelope");
    let index = container.program(ByteOffset(0)).expect("fixture index");
    for ordinal in 0..index.class_count() {
        let key = ClassKey {
            dex: DexId(0),
            class_def: diana_decomp::ClassDefId(ordinal),
        };
        let facts = engine.class_facts(key).expect("fixture class facts");
        if facts
            .descriptor()
            .units()
            .iter()
            .copied()
            .eq(descriptor.encode_utf16())
        {
            return key;
        }
    }
    panic!("fixture class not found: {descriptor}")
}
