mod support;

use diana_decomp::Request;

const DEX: &[u8] =
    include_bytes!("../../../quality/fixtures/atoms/static-field-values/classes.dex");
const WIDTHS: &[u8] =
    include_bytes!("../../../quality/fixtures/atoms/static-field-value-widths/classes.dex");

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
        "int ZERO = 0;",
        "int[] SEQUENCE = new int[] { -7, -300, -1234567, 6 };",
    ] {
        assert!(
            text.contains(expected),
            "missing original {expected}: {text}"
        );
    }
}
