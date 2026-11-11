//! Opt-in request counters for the `perf` feature.
//!
//! These counters complement fastrace's coarse spans. They are intentionally
//! aggregated: per-method spans would distort the workload and overwhelm the
//! timeline reporter.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

pub(crate) static REQUEST_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static SCAN_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static LOGICAL_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static DECLARATION_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static CODE_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static PREFILTER_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static FLAT_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static FALLBACK_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static ENCODE_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static RANK_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static BODY_READ_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static BYTE_FILTER_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static BATCH_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static BATCH_REFERENCE_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static BATCH_DEDUP_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static BATCH_CONTEXT_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static CLASS_WALK_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static CLASS_DATA_PARSE_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static ANNOTATION_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static STATIC_VALUES_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static TYPE_FIELD_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static TYPE_METHOD_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static CALLBACK_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static DECLARATION_CONTEXT_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static ANNOTATION_DIRECTORY_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static ANNOTATION_OFFSETS_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static ANNOTATION_MEMBER_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static ANNOTATION_MEMBER_OPEN_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static ANNOTATION_HEADER_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static ANNOTATION_VALUES_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static DECOMPILE_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static RESOLVE_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static IMAGE_BYTES_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static ENGINE_OPEN_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static CLASS_FACTS_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static METHOD_SELECT_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static ANALYZE_NS: AtomicU64 = AtomicU64::new(0);
pub(crate) static PUBLISH_NS: AtomicU64 = AtomicU64::new(0);

pub(crate) static LOGICALS: AtomicU64 = AtomicU64::new(0);
pub(crate) static CLASSES: AtomicU64 = AtomicU64::new(0);
pub(crate) static BODIES: AtomicU64 = AtomicU64::new(0);
pub(crate) static SKIPPED: AtomicU64 = AtomicU64::new(0);
pub(crate) static FLAT: AtomicU64 = AtomicU64::new(0);
pub(crate) static FALLBACK: AtomicU64 = AtomicU64::new(0);
pub(crate) static SITES: AtomicU64 = AtomicU64::new(0);
pub(crate) static HITS: AtomicU64 = AtomicU64::new(0);
pub(crate) static DECLARATION_CLASSES: AtomicU64 = AtomicU64::new(0);
pub(crate) static ANNOTATED_CLASSES: AtomicU64 = AtomicU64::new(0);
pub(crate) static VALUE_CLASSES: AtomicU64 = AtomicU64::new(0);
pub(crate) static ANNOTATION_SETS: AtomicU64 = AtomicU64::new(0);
pub(crate) static STATIC_VALUE_CLASSES: AtomicU64 = AtomicU64::new(0);
pub(crate) static TYPE_FIELDS: AtomicU64 = AtomicU64::new(0);
pub(crate) static TYPE_METHODS: AtomicU64 = AtomicU64::new(0);
pub(crate) static CLASS_DATA_MEMBERS: AtomicU64 = AtomicU64::new(0);
pub(crate) static ENCODED_ROWS: AtomicU64 = AtomicU64::new(0);

const COUNTERS: &[&AtomicU64] = &[
    &REQUEST_NS,
    &SCAN_NS,
    &LOGICAL_NS,
    &DECLARATION_NS,
    &CODE_NS,
    &PREFILTER_NS,
    &FLAT_NS,
    &FALLBACK_NS,
    &ENCODE_NS,
    &RANK_NS,
    &BODY_READ_NS,
    &BYTE_FILTER_NS,
    &BATCH_NS,
    &BATCH_REFERENCE_NS,
    &BATCH_DEDUP_NS,
    &BATCH_CONTEXT_NS,
    &CLASS_WALK_NS,
    &CLASS_DATA_PARSE_NS,
    &ANNOTATION_NS,
    &STATIC_VALUES_NS,
    &TYPE_FIELD_NS,
    &TYPE_METHOD_NS,
    &CALLBACK_NS,
    &DECLARATION_CONTEXT_NS,
    &ANNOTATION_DIRECTORY_NS,
    &ANNOTATION_OFFSETS_NS,
    &ANNOTATION_MEMBER_NS,
    &ANNOTATION_MEMBER_OPEN_NS,
    &ANNOTATION_HEADER_NS,
    &ANNOTATION_VALUES_NS,
    &DECOMPILE_NS,
    &RESOLVE_NS,
    &IMAGE_BYTES_NS,
    &ENGINE_OPEN_NS,
    &CLASS_FACTS_NS,
    &METHOD_SELECT_NS,
    &ANALYZE_NS,
    &PUBLISH_NS,
    &LOGICALS,
    &CLASSES,
    &BODIES,
    &SKIPPED,
    &FLAT,
    &FALLBACK,
    &SITES,
    &HITS,
    &DECLARATION_CLASSES,
    &ANNOTATED_CLASSES,
    &VALUE_CLASSES,
    &ANNOTATION_SETS,
    &STATIC_VALUE_CLASSES,
    &TYPE_FIELDS,
    &TYPE_METHODS,
    &CLASS_DATA_MEMBERS,
];

pub(crate) fn reset() {
    for counter in COUNTERS {
        counter.store(0, Ordering::Relaxed);
    }
    ENCODED_ROWS.store(0, Ordering::Relaxed);
}

#[inline]
pub(crate) fn add(counter: &AtomicU64, value: u64) {
    counter.fetch_add(value, Ordering::Relaxed);
}

#[inline]
pub(crate) fn add_elapsed(counter: &AtomicU64, started: Instant) {
    add(
        counter,
        u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX),
    );
}

pub(crate) struct Timer {
    started: Instant,
    counter: &'static AtomicU64,
}

impl Timer {
    pub(crate) fn new(counter: &'static AtomicU64) -> Self {
        Self {
            started: Instant::now(),
            counter,
        }
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        add_elapsed(self.counter, self.started);
    }
}

pub(crate) fn print(scope: &str) {
    let value = |counter: &AtomicU64| counter.load(Ordering::Relaxed);
    let ms = |counter: &AtomicU64| value(counter) as f64 / 1_000_000.0;
    let mut report = serde_json::Map::new();
    macro_rules! field {
        ($key:literal, $value:expr) => {
            report.insert($key.to_owned(), serde_json::Value::from($value));
        };
    }
    field!("record", "perf");
    field!("scope", scope);
    field!("request_ms", ms(&REQUEST_NS));
    field!("scan_ms", ms(&SCAN_NS));
    field!("logical_ms", ms(&LOGICAL_NS));
    field!("declaration_ms", ms(&DECLARATION_NS));
    field!("code_ms", ms(&CODE_NS));
    field!("prefilter_ms", ms(&PREFILTER_NS));
    field!("flat_ms", ms(&FLAT_NS));
    field!("fallback_ms", ms(&FALLBACK_NS));
    field!("encode_ms", ms(&ENCODE_NS));
    field!("rank_ms", ms(&RANK_NS));
    field!("body_read_ms", ms(&BODY_READ_NS));
    field!("byte_filter_ms", ms(&BYTE_FILTER_NS));
    field!("batch_ms", ms(&BATCH_NS));
    field!("batch_reference_ms", ms(&BATCH_REFERENCE_NS));
    field!("batch_dedup_ms", ms(&BATCH_DEDUP_NS));
    field!("batch_context_ms", ms(&BATCH_CONTEXT_NS));
    field!("class_walk_ms", ms(&CLASS_WALK_NS));
    field!("class_data_parse_ms", ms(&CLASS_DATA_PARSE_NS));
    field!("annotation_ms", ms(&ANNOTATION_NS));
    field!("static_values_ms", ms(&STATIC_VALUES_NS));
    field!("type_field_ms", ms(&TYPE_FIELD_NS));
    field!("type_method_ms", ms(&TYPE_METHOD_NS));
    field!("callback_ms", ms(&CALLBACK_NS));
    field!("declaration_context_ms", ms(&DECLARATION_CONTEXT_NS));
    field!("annotation_directory_ms", ms(&ANNOTATION_DIRECTORY_NS));
    field!("annotation_offsets_ms", ms(&ANNOTATION_OFFSETS_NS));
    field!("annotation_member_ms", ms(&ANNOTATION_MEMBER_NS));
    field!("annotation_member_open_ms", ms(&ANNOTATION_MEMBER_OPEN_NS));
    field!("annotation_header_ms", ms(&ANNOTATION_HEADER_NS));
    field!("annotation_values_ms", ms(&ANNOTATION_VALUES_NS));
    field!("decompile_ms", ms(&DECOMPILE_NS));
    field!("resolve_ms", ms(&RESOLVE_NS));
    field!("image_bytes_ms", ms(&IMAGE_BYTES_NS));
    field!("engine_open_ms", ms(&ENGINE_OPEN_NS));
    field!("class_facts_ms", ms(&CLASS_FACTS_NS));
    field!("method_select_ms", ms(&METHOD_SELECT_NS));
    field!("analyze_ms", ms(&ANALYZE_NS));
    field!("publish_ms", ms(&PUBLISH_NS));
    field!("logicals", value(&LOGICALS));
    field!("classes", value(&CLASSES));
    field!("bodies", value(&BODIES));
    field!("skipped", value(&SKIPPED));
    field!("flat", value(&FLAT));
    field!("fallback", value(&FALLBACK));
    field!("sites", value(&SITES));
    field!("hits", value(&HITS));
    field!("declaration_classes", value(&DECLARATION_CLASSES));
    field!("annotated_classes", value(&ANNOTATED_CLASSES));
    field!("value_classes", value(&VALUE_CLASSES));
    field!("annotation_sets", value(&ANNOTATION_SETS));
    field!("static_value_classes", value(&STATIC_VALUE_CLASSES));
    field!("type_fields", value(&TYPE_FIELDS));
    field!("type_methods", value(&TYPE_METHODS));
    field!("class_data_members", value(&CLASS_DATA_MEMBERS));
    field!("encoded_rows", value(&ENCODED_ROWS));
    eprintln!("{}", serde_json::Value::Object(report));
}
