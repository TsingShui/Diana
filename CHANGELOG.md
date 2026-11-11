# Changelog

## 0.2.6 - 2026-11-11

### Decompilation

- Restored original encoded static field values instead of silently rendering
  descriptor defaults. Scalar widths, negative zero and UTF-16 string units are
  preserved.
- Allocation and direct constructor calls with different owners now remain
  separate `allocate-instance` and `init-direct` operations, preserving the
  original constructor target, execution sites and surrounding effects.
- Accepted DEX map kind `0xF000` as opaque platform hidden-API metadata under
  the existing map envelope checks, allowing affected framework DEX images to
  reach normal analysis.
- Unavailable method bodies now contain explicit Diana warning comments with
  the recovery reason and issue URL. The diagnostic no longer invents an
  executable exception throw; original input throws remain readable code.

### CLI compatibility

- `get-class` and `get-method` JSON publications now use schema 5 and include
  structured method-local `issues` with the original identity, stage and reason.
- Method-local recovery failures publish the available result, report a
  diagnostic on stderr and return exit code 1 in both text and JSON formats.
  Output selected with `-o` remains available after a recovery failure.

### Documentation

- Retired obsolete pipeline specifications and references in the active
  contribution and package documentation.

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
