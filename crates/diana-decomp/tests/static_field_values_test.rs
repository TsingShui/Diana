mod support;

use diana_decomp::{
    p0::{Fact, InitialValue},
    Request,
};

const DEX: &[u8] =
    include_bytes!("../../../quality/fixtures/atoms/static-field-values/classes.dex");
const WIDTHS: &[u8] =
    include_bytes!("../../../quality/fixtures/atoms/static-field-value-widths/classes.dex");
const RUNTIME: &[u8] =
    include_bytes!("../../../quality/fixtures/classes/runtime-static-final/classes.dex");
const OWNERS: &[u8] =
    include_bytes!("../../../quality/fixtures/classes/static-initializer-owners/classes.dex");
const PREPARED: &[u8] =
    include_bytes!("../../../quality/fixtures/classes/prepared-static-final/classes.dex");

#[test]
fn renders_original_static_field_preparations() {
    let engine = support::engine(DEX);
    let class = support::class(DEX, &engine, "Lfixture/StaticFieldValues;");
    let analysis = engine
        .analyze(Request {
            class,
            method: None,
        })
        .expect("class analysis");
    let text = analysis.rendered.text();

    for (name, value) in [
        ("CLOSE", "5"),
        ("PREPARE_CAPTURE", "3"),
        ("PREPARE_PREVIEW", "1"),
        ("PROCESS_CAPTURE_IMAGE", "4"),
        ("PROCESS_PREVIEW", "2"),
    ] {
        let expected = format!("static final int {name} = {value};");
        assert!(text.contains(&expected), "missing {expected}\n{text}");
    }
    assert!(
        text.contains("static final String LABEL = \"fixture\";"),
        "missing string preparation\n{text}"
    );
}

#[test]
fn mixed_width_preparations_preserve_values_and_class_initialization() {
    let engine = support::engine(WIDTHS);
    let class = support::class(WIDTHS, &engine, "Lfixture/StaticFieldValueWidths;");
    let facts = engine.class_facts(class).expect("original field facts");
    let zero = facts
        .fields()
        .iter()
        .find(|entry| {
            matches!(entry.declaration(), Fact::Present(field)
                if field.name.units().iter().copied().eq("ZERO".encode_utf16()))
        })
        .expect("original ZERO field");
    assert!(matches!(
        zero.initial_value(),
        Fact::Present(InitialValue::ImplicitDefault)
    ));
    let analysis = engine
        .analyze(Request {
            class,
            method: None,
        })
        .expect("mixed preparations");
    assert!(analysis.failures.is_empty());
    let text = analysis.rendered.text();
    for expected in [
        "boolean BOOL = true;",
        "byte BYTE = -7;",
        "char CHAR = 'Q';",
        "double DOUBLE = -2.5d;",
        "int END = 6;",
        "float FLOAT = 1.25f;",
        "int INT = -1234567;",
        "String LABEL = \"tail\";",
        "long LONG = 1234605616436508552L;",
        "float NEGATIVE_ZERO = -0.0f;",
        "short SHORT = -300;",
        "String SURROGATE = \"\\ud800\";",
        "int ZERO = null == null ? 0 : 0;",
        "int[] SEQUENCE = new int[] { -7, -300, -1234567, 6 };",
    ] {
        assert!(
            text.contains(expected),
            "missing original {expected}: {text}"
        );
    }
}

#[test]
fn runtime_final_writes_stay_before_later_class_initialization() {
    let engine = support::engine(RUNTIME);
    let class = support::class(RUNTIME, &engine, "Lfixture/DexProvider;");
    let analysis = engine
        .analyze(Request {
            class,
            method: None,
        })
        .expect("class analysis");
    assert!(analysis.failures.is_empty());
    let text = analysis.rendered.text();
    assert!(
        text.contains("static final String COMPILE_TEXT = \"compiled\";"),
        "{text}"
    );
    assert!(text.contains("static final String RUNTIME_TEXT;"), "{text}");
    let mut remaining = text;
    for marker in [
        "static {",
        "RUNTIME_TEXT = \"runtime\";",
        "RUNTIME_NUMBER = 7;",
        "DexInitEvents.begin(",
        "DexInitEvents.complete()",
    ] {
        remaining = remaining
            .split_once(marker)
            .unwrap_or_else(|| panic!("missing {marker}:\n{text}"))
            .1;
    }
}

#[test]
fn static_initializers_preserve_foreign_members_with_the_same_owner_name() {
    let engine = support::engine(OWNERS);
    for (owner, expected) in [
        ("Lorigin/PrefixOwner;", "outside.PrefixOwner.value = 7;"),
        ("Lorigin/LateOwner;", "outside.LateOwner.value = 5;"),
    ] {
        let class = support::class(OWNERS, &engine, owner);
        let analysis = engine
            .analyze(Request {
                class,
                method: None,
            })
            .expect("class analysis");
        assert!(analysis.failures.is_empty());
        let text = analysis.rendered.text();
        assert!(text.contains("static int value;"), "{text}");
        assert!(
            text.split_once("static {")
                .is_some_and(|(_, body)| body.contains(expected)),
            "{text}"
        );
    }
}

#[test]
fn preparation_only_static_finals_keep_active_reads() {
    let engine = support::engine(PREPARED);
    let render = |descriptor| {
        let class = support::class(PREPARED, &engine, descriptor);
        let analysis = engine
            .analyze(Request {
                class,
                method: None,
            })
            .expect("prepared static field fixture");
        assert!(analysis.failures.is_empty());
        analysis.rendered.text().to_owned()
    };

    let fields = render("Lfixture/PreparedDefaults;");
    for expected in [
        "class PreparedDefaults extends PreparationParent",
        "static final int A_ENCODED_ZERO = 0;",
        "static final int B_ENCODED_SENTINEL = 9;",
        "static final int C_PREPARED_ZERO = null == null ? 0 : 0;",
    ] {
        assert!(fields.contains(expected), "missing {expected}: {fields}");
    }

    let readers = render("Lfixture/ReadPreparedDefaults;");
    for expected in [
        "return 0;",
        "return PreparedDefaults.C_PREPARED_ZERO;",
        "return PreparedDefaults.D_PREPARED_NULL;",
    ] {
        assert!(readers.contains(expected), "missing {expected}: {readers}");
    }
    let parent = render("Lfixture/PreparationParent;");
    assert!(
        parent
            .split_once("PreparationEvents.start();")
            .is_some_and(|(_, tail)| tail.contains("PreparationEvents.finish();")),
        "{parent}"
    );
}
