mod support;

use diana_decomp::Request;

const DEX: &[u8] =
    include_bytes!("../../../quality/fixtures/sync/monitor-execution-contract/classes.dex");

#[test]
fn physical_monitor_scopes_and_declared_flags_keep_their_original_roles() {
    let engine = support::engine(DEX);
    let class = support::class(DEX, &engine, "Lfixture/MonitorContract;");
    let facts = engine.class_facts(class).expect("original class facts");
    for method in [1, 5] {
        let entry = facts
            .methods()
            .iter()
            .find(|entry| entry.method().0 == method)
            .expect("original method identity");
        assert_eq!(entry.access_flags().raw(), 0x20001);
    }

    let analysis = engine
        .analyze(Request {
            class,
            method: None,
        })
        .expect("class analysis");
    assert!(analysis.failures.is_empty(), "{:?}", analysis.failures);
    let text = analysis.rendered.text();
    for declaration in [
        "public boolean metadataOnly()",
        "public boolean physical(boolean ",
        "native synchronized void nativeSync()",
    ] {
        assert!(text.contains(declaration), "missing {declaration}:\n{text}");
    }
    assert_eq!(text.matches("synchronized (this)").count(), 3, "{text}");
    assert_eq!(text.matches("synchronized (object)").count(), 2, "{text}");
}
