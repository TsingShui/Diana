# Changelog

## 0.2.5 - 2026-11-11

### Performance

- Cross-reference scans now use target-index prefilters before resolving
  annotations, static values, field types, and method return types.
- Field and method annotation association avoids resolving complete member
  declarations when the query only needs the declaring member identity.
- Type queries can answer return-type and field-type relations without
  materializing unrelated declarations.
- Code-reference batches reuse resolved targets and method contexts while
  preserving the first matching use in each method.

### Instrumentation

- Added an opt-in `perf` feature with fastrace spans and aggregated xref,
  annotation, class-data, batch, resolve, analyze, and publish counters.
  `app_profile` is the development entry point; normal release builds are
  unchanged.
- Removed the unused no-op `profile_scope!` compatibility macro and
  `crates/diana-decomp/src/profiling.rs`.
