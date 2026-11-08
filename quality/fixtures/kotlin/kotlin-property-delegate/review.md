# Fixture: kotlin-property-delegate

delegated properties, accessor pair and lateinit. The Kotlin source is the semantic reference for the LLM review below.

- Provenance: authored for this repository. No third-party code or data.
- Prepared with `kotlinc -jvm-target 1.8` + `d8 --min-api 24 --lib kotlin-stdlib.jar`
  (see `quality/build.sh` output for the versions in use).
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/Case;`

## Current verdict — round 1, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `Case` | ? Not judged. |
| `0` | ? Not judged. |
| `getCached` | ? Not reviewed line by line. |
| `cache` | ✔ getCached() is used and the cached$delegate field is retained. |
| `getLogged` | ? Not reviewed line by line. |
| `getObserved` | ? Not reviewed line by line. |
| `getReady` | ? Not judged. |
| `prepare` | ✔ setReady("ready") agrees. |
| `readyLength` | ✔ The null check correctly represents the lowered ready.isInitialized test. |
| `setLogged` | ? Not reviewed line by line. |
| `setObserved` | ? Not reviewed line by line. |
| `setReady` | ? Not judged. |

### Findings

- Delegated properties expand into delegate fields, LazyKt.lazy(lambda) and ReadWriteProperty calls; the behavior remains but source syntax is absent. Recorded forms: `$delegate`.
- The delegated-properties array is built through Reflection.mutableProperty1 and MutablePropertyReference1Impl; allocation and constructor invocation are split. Recorded forms: `$$delegatedProperties`, `Reflection.mutableProperty1(new MutablePropertyReference1Impl(...))`, `new X;`, `.<init>`.
- Signature metadata appears in the body; lateinit expands into a nullable field and null check. Recorded forms: `@Signature(KProperty<Object>[]) system`.

### Ranked fixes

Improve delegated-property and constructor presentation while preserving metadata and initialization facts. Recorded forms: `by lazy`, `by Delegate`, `new X(...)`.

## Rounds

- R1 2026-11-08: Delegated properties expand into delegate fields, LazyKt.lazy(lambda) and ReadWriteProperty calls; the behavior remains but source syntax is absent. Recorded forms: `$delegate`.
