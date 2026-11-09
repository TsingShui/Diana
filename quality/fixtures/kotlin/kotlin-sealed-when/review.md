# Fixture: kotlin-sealed-when

## Current verdict — scalar literals and retained dispatch

Compared df10b74 with this rendering change on the same DEX. Only generated
Ready/Failed toString's char41 operands change to ')'. Original class/member
identities, prefix/value/reason appends and char signature qualifiers remain.
P2–P8 products agree exactly; other member text is unchanged. Earlier bounded
judgments and their unreviewed limits below are reused, not execution results.

### Semantics

| Method | Judgment and scope |
|---|---|
| `Ready.toString` | ~ The unchanged generated prefix and int value are followed by the same closing UTF-16 unit41. |
| `Failed.toString` | ~ The unchanged generated prefix and reason are followed by that same closing unit. |
| `Case.describe`, `Case.labelled`, generated equals methods | ~ Unchanged text/products; previous dispatch/guard path judgments are reused. |

### Reused guard evidence

Read unknown-variant throws before zero/pending assignment and every changed
equals method. Identity/membership/field comparisons retain order; a failed
guard skips all subsequent reads. Original when/dispatch and literal values
remain.

The original guard review compared the same DEX on 29f1b82 and 7354242.
That evidence is reused for the unchanged members. P7 requires a complete
linear arm ending in a printed return/throw; opaque scopes are not exit evidence.
The [fresh guard challenge](../../branches/terminal-guard-arms/review.md) covers
actual other-edge Phi copies, parent scope, recovering catches and cleanup.
Original P2 through complete P6 products agree; this is bounded LLM path reading,
not behavioral execution. Historical execution claims below were not rerun.


sealed hierarchy with exhaustive when plus smart casts. The Kotlin source is the semantic reference for the LLM review below.

- Provenance: authored for this repository. No third-party code or data.
- Prepared with `kotlinc -jvm-target 1.8` + `d8 --min-api 24 --lib kotlin-stdlib.jar`
  (see `quality/build.sh` output for the versions in use).
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/Case;`

## Prior verdict — round 1, 2026-11-08

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

- Scalar-use disclosure: reread Ready/Failed toString closing units; original dispatch, guards and all other text remain unchanged.
- R1 2026-11-08: when becomes nested negated instanceof tests with a leading nop. Recorded forms: `when (this) { is Ready -> … }`, `if (!(x instanceof Ready))`, `nop;`.
