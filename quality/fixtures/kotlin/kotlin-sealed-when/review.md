# Fixture: kotlin-sealed-when

sealed hierarchy with exhaustive when plus smart casts. The Kotlin source is the semantic reference for the LLM review below.

- Provenance: authored for this repository. No third-party code or data.
- Prepared with `kotlinc -jvm-target 1.8` + `d8 --min-api 24 --lib kotlin-stdlib.jar`
  (see `quality/build.sh` output for the versions in use).
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/Case;`

## Current verdict — round 1, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `Case` | ✔ The synthetic Case(DefaultConstructorMarker) constructor is retained. |
| `describe` | ✔ The instanceof chain and NoWhenBranchMatchedException preserve exhaustive selection. |
| `labelled` | ✔ All three branches and string templates agree. |

### Findings

- when becomes nested negated instanceof tests with a leading nop. Recorded forms: `when (this) { is Ready -> … }`, `if (!(x instanceof Ready))`, `nop;`.
- Ready/Failed/Pending are separate classes; MemberClasses retains their relationship. Recorded forms: `@MemberClasses`.
- NoWhenBranchMatchedException retains the exhaustiveness fact without reconstructing the source when form.

### Ranked fixes

Improve when presentation and unnecessary test/nop notation while retaining type-selection facts. Recorded forms: `test {}`, `nop;`.

## Rounds

- R1 2026-11-08: when becomes nested negated instanceof tests with a leading nop. Recorded forms: `when (this) { is Ready -> … }`, `if (!(x instanceof Ready))`, `nop;`.
