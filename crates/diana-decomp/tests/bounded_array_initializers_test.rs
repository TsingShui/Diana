mod support;

use diana_decomp::{MethodSelector, Request};

const DEX: &[u8] =
    include_bytes!("../../../quality/fixtures/atoms/bounded-array-initializers/classes.dex");

const ALLOCATION_ORDER: &[u8] =
    include_bytes!("../../../quality/fixtures/atoms/filled-array-order/classes.dex");

#[test]
fn array_initializers_use_only_the_recovered_stores() {
    #[cfg(target_os = "linux")]
    if std::env::var_os("DIANA_ARRAY_FIXTURE_LIMITED").is_none() {
        // Linux can overcommit a large reservation without increasing RSS. Run
        // the same Engine request in a child with a deterministic address limit.
        let result = std::process::Command::new("sh")
            .args([
                "-c",
                r#"ulimit -c 0 && ulimit -v 524288 && exec "$@""#,
                "array-fixture",
            ])
            .arg(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "array_initializers_use_only_the_recovered_stores",
            ])
            .env("DIANA_ARRAY_FIXTURE_LIMITED", "1")
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        return;
    }

    let engine = support::engine(DEX).with_method_jobs(1);
    let class = support::class(DEX, &engine, "Lfixture/LargeArray;");
    let output = engine
        .class_source(Request {
            class,
            method: None,
        })
        .unwrap();
    assert!(output.failures.is_empty(), "{:?}", output.failures);
    let source = &output.text;
    assert!(source.contains("new int[] { i, i2 }"));
    assert!(source.contains("new int[2];\n        iArr[0] = i;\n        iArr[1] = iArr[0];"));
    assert!(source.contains("new int[2147483647];\n        iArr[0] = i;"));
    assert_eq!(source.matches("return iArr;").count(), 3);
}

#[test]
fn keeps_precomputed_elements_before_array_allocation() {
    let engine = support::engine(ALLOCATION_ORDER);
    let source = |owner, name: &str, prototype: &str| {
        let class = support::class(ALLOCATION_ORDER, &engine, owner);
        let output = engine
            .class_source(Request {
                class,
                method: Some(MethodSelector {
                    name: name.encode_utf16().collect(),
                    prototype: prototype.encode_utf16().collect(),
                }),
            })
            .unwrap();
        assert!(output.failures.is_empty(), "{:?}", output.failures);
        output.text
    };

    let filled = source("Lfixture/ArrayOrder;", "filled", "()[I");
    let (before, after) = filled.split_once("new int[] {").expect("array initializer");
    assert!(
        before.contains("ArrayOrder.next(1)") && before.contains("ArrayOrder.next(2)"),
        "{filled}"
    );
    assert!(!after.contains("ArrayOrder.next("), "{filled}");

    let stored = source("Lfixture/ArrayOrder;", "stored", "()[I");
    assert!(
        stored.contains("new int[] { ArrayOrder.next(1), ArrayOrder.next(2) }"),
        "{stored}"
    );

    let protected = source(
        "Lfixture/ArrayOrderProtected;",
        "protectedFilled",
        "()[Ljava/lang/Object;",
    );
    let (before, handler) = protected
        .split_once("catch (RuntimeException")
        .expect("typed handler");
    let (prepared, initializer) = before
        .split_once("new Object[] {")
        .expect("array initializer");
    assert!(
        prepared.contains("ArrayOrderProtected.make(20)") && !initializer.contains(".make("),
        "{protected}"
    );
    assert!(
        handler.contains("ArrayOrderProtected.caught(80,"),
        "{protected}"
    );
}
