mod support;

use diana_decomp::Request;

const DEX: &[u8] =
    include_bytes!("../../../quality/fixtures/classes/interface-holder-publication/classes.dex");

fn class_text(descriptor: &str) -> String {
    let engine = support::engine(DEX);
    let class = support::class(DEX, &engine, descriptor);
    let result = engine
        .analyze(Request {
            class,
            method: None,
        })
        .expect("interface initialization source");
    assert!(result.failures.is_empty(), "{:?}", result.failures);
    result.rendered.text().to_owned()
}

#[test]
fn callback_observes_the_first_interface_field_commit() {
    let text = class_text("Lfixture/PublicationFace;");
    assert!(!text.contains("DexdecStaticInitialization"), "{text}");
    let (_, body) = text.split_once("static {").expect(&text);
    let (before_first, after_first) = body.split_once("FIRST = ").expect(&text);
    let (saved, after_first) = after_first.split_once(';').expect(&text);
    assert!(
        before_first.contains(&format!("Object {saved} = new Object();")),
        "{text}"
    );
    let (_, after_callback) = after_first
        .split_once("PublicationFace.observe();")
        .expect(&text);
    assert!(
        after_callback.contains(&format!("SECOND = {saved};")),
        "{text}"
    );
    assert!(
        text.contains("HolderObservation.seen = PublicationFace.FIRST;")
            && text.matches("new Object()").count() == 1,
        "{text}"
    );
}

#[test]
fn shared_local_tail_without_a_callback_still_uses_a_holder() {
    let text = class_text("Lfixture/HolderTail;");
    assert!(
        text.contains("Object FIRST = DexdecStaticInitialization.FIRST;")
            && text.contains("Object SECOND = DexdecStaticInitialization.SECOND;"),
        "{text}"
    );
    assert_eq!(text.matches("new Object()").count(), 1, "{text}");
    let (_, after_first) = text
        .split_once("HolderTail.DexdecStaticInitialization.FIRST = ")
        .expect(&text);
    let (saved, after_first) = after_first.split_once(';').expect(&text);
    assert!(
        after_first.contains(&format!(
            "HolderTail.DexdecStaticInitialization.SECOND = {saved};"
        )),
        "{text}"
    );
}
