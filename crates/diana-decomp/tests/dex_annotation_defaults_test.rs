mod support;

use diana_decomp::Request;

const DEX: &[u8] =
    include_bytes!("../../../quality/fixtures/classes/annotation-default-values/classes.dex");

#[test]
fn dex_annotation_defaults_attach_to_original_member_names() {
    let engine = support::engine(DEX);
    let class = support::class(DEX, &engine, "Lfixture/DexDefaults;");
    let analysis = engine
        .analyze(Request {
            class,
            method: None,
        })
        .expect("annotation interface analysis");
    assert!(analysis.failures.is_empty(), "{:?}", analysis.failures);
    let text = analysis.rendered.text();
    for declaration in [
        "int count() default -33;",
        "boolean enabled() default true;",
        "labels() default ",
        "int required();",
    ] {
        assert!(text.contains(declaration), "missing {declaration}\n{text}");
    }
    assert!(
        !text.contains("dalvik.annotation.AnnotationDefault"),
        "{text}"
    );

    let method = analysis
        .facts
        .resolve_method(
            &"count".encode_utf16().collect::<Vec<_>>(),
            &"()I".encode_utf16().collect::<Vec<_>>(),
        )
        .expect("original count()I identity");
    let selected = engine
        .analyze_method(class, method)
        .expect("selected annotation member analysis");
    assert!(selected.failures.is_empty(), "{:?}", selected.failures);
    let text = selected.rendered.text();
    assert!(text.contains("int count() default -33;"), "{text}");
}
