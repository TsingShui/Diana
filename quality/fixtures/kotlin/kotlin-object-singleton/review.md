# Fixture: kotlin-object-singleton

object singleton and companion object. The Kotlin source is the semantic reference for the LLM review below.

- Provenance: authored for this repository. No third-party code or data.
- Prepared with `kotlinc -jvm-target 1.8` + `d8 --min-api 24 --lib kotlin-stdlib.jar`
  (see `quality/build.sh` output for the versions in use).
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/Counter;`

## Current verdict: companion string preparation

Compared the same prepared DEX on `b5eb94d` and the recovery-fixes candidate.
Only Case.PREFIX changes: the candidate retains Kotlin's `case:` preparation
instead of null. Counter and all method bodies remain byte-for-byte unchanged;
the bounded prior judgments below are reused for unchanged content. This is LLM
semantic review, not an executed Kotlin/ART behavioral check.

### Semantics

| Subject | Judgment and reasoning |
|---|---|
| `Case.PREFIX` | Agreement: Kotlin declares the constant `case:`. The field now retains that value without adding an initializer call or altering companion creation. |

## Prior verdict — round 1, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `Counter` | ✔ INSTANCE, the private constructor and static block retain the lowered Kotlin object form. |
| `add` | ✔ Static total accumulation agrees. |
| `getTotal` | ✔ |

### Findings

- The object INSTANCE field and static construction are retained. Recorded forms: `public static final Counter INSTANCE`, `new Counter()`.
- Companion initialization exposes static_preparation_before_clinit, a null assignment and implicit_default notation. Recorded forms: `static_preparation_before_clinit { Companion = null; PREFIX = "case:"; created = implicit_default; }`.
- Synthetic accessors access$getCreated$cp / access$setCreated$cp appear as ordinary methods.

### Ranked fixes

Improve companion constants and initialization notation while retaining original initialization facts. Recorded forms: `static_preparation_before_clinit`, `implicit_default`.

## Rounds

- R1 2026-11-08: The object INSTANCE field and static construction are retained. Recorded forms: `public static final Counter INSTANCE`, `new Counter()`.
