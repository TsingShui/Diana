mod support;

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use diana_decomp::{ClassDefId, DexId, RequestError};

const MANY_METHODS: &[u8] =
    include_bytes!("../../../quality/fixtures/generics/complex-stream/classes.dex");
const PARTIAL: &[u8] =
    include_bytes!("../../../quality/fixtures/classes/recovery-warning-members/classes.dex");

fn sources(bytes: &[u8], jobs: usize) -> BTreeMap<ClassDefId, serde_json::Value> {
    let engine = support::engine(bytes);
    let outputs = Mutex::new(BTreeMap::new());
    engine
        .visit_class_sources(DexId(0), jobs, |class, result| {
            let (descriptor, source) = result?;
            let methods = source
                .methods
                .into_iter()
                .map(|method| {
                    serde_json::json!([
                        method.method.0,
                        method.name,
                        method.prototype,
                        method.failure
                    ])
                })
                .collect::<Vec<_>>();
            let result = serde_json::json!([descriptor, source.text, methods, source.failures]);
            assert!(outputs.lock().unwrap().insert(class, result).is_none());
            Ok::<(), RequestError>(())
        })
        .unwrap();
    outputs.into_inner().unwrap()
}

#[test]
fn bulk_workers_preserve_each_definition_method_and_failure() {
    for (dex, count) in [(MANY_METHODS, 9), (PARTIAL, 3)] {
        let sequential = sources(dex, 1);
        assert_eq!(sequential.len(), count);
        assert_eq!(sources(dex, 4), sequential);
        if dex == PARTIAL {
            let class = support::class(
                dex,
                &support::engine(dex),
                "Lfixture/RecoveryWarningMembers;",
            );
            assert!(!sequential[&class.class_def][3]
                .as_array()
                .unwrap()
                .is_empty());
        }
    }
}

#[test]
fn sink_failure_limits_in_flight_class_requests() {
    for jobs in [1, 2, 4] {
        let engine = support::engine(MANY_METHODS);
        let calls = AtomicUsize::new(0);
        let failure = engine.visit_class_sources(DexId(0), jobs, |_, _| {
            calls.fetch_add(1, Ordering::Relaxed);
            Err::<(), RequestError>(RequestError::InternalInvariant("sink stopped"))
        });
        assert!(matches!(
            failure,
            Err(RequestError::InternalInvariant("sink stopped"))
        ));
        assert!((1..=jobs).contains(&calls.into_inner()));
    }
}
