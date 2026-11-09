# Fixture: kotlin-data-class

## Current verdict — scalar literals and retained guards

Compared df10b74 with this rendering change on the same DEX. Only describe's
char58 and toString's closing char41 change to ':' and ')'. Original append(C)
bindings, field reads and StringBuilder call order remain. P2–P8 products agree
exactly. The other member text is unchanged; earlier judgments and their
unreviewed limits below are reused, not promoted to execution results.

### Semantics

| Method | Judgment and scope |
|---|---|
| `describe` | ~ Name append precedes the exact colon unit, then count append; the original char signature qualifier remains. |
| `toString` | ~ Generated prefix/name/count sequence is unchanged, followed by the same closing parenthesis unit. |
| `equals` | ~ Unchanged text/products; the prior short-circuit guard reading is reused. |

### Reused guard evidence

Read equals: identity returns before instanceof, failed membership returns
before field reads, unequal names return before count reads, then the original
count comparison selects true/false. Original checks and owner disclosure
remain, without inferred hierarchy.

The original guard review compared the same DEX on 29f1b82 and 7354242.
That evidence is reused for the unchanged members. P7 requires a complete
linear arm ending in a printed return/throw; opaque scopes are not exit evidence.
The [fresh guard challenge](../../branches/terminal-guard-arms/review.md) covers
actual other-edge Phi copies, parent scope, recovering catches and cleanup.
Original P2 through complete P6 products agree; this is bounded LLM path reading,
not behavioral execution. Historical execution claims below were not rerun.


data class and the members kotlinc generates (componentN, copy, copy$default, equals/hashCode/toString). The Kotlin source is the semantic reference for the LLM review below.

- Provenance: authored for this repository. No third-party code or data.
- Prepared with `kotlinc -jvm-target 1.8` + `d8 --min-api 24 --lib kotlin-stdlib.jar`
  (see `quality/build.sh` output for the versions in use).
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/Case;`

## Prior verdict — round 1, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `Case` | ? Not judged. |
| `default` | ? Not judged. |
| `component1` | ? Not reviewed line by line. |
| `component2` | ? Not reviewed line by line. |
| `copy` | ? Not judged. |
| `describe` | ✔ The behavior agrees; ':' appears as append<(char)>(58). |
| `equals` | ? Not reviewed line by line. |
| `first` | ✔ The access is recovered as component1(). |
| `getCount` | ✔ |
| `getName` | ✔ |
| `hashCode` | ? Not reviewed line by line. |
| `increase` | ✔ The behavior agrees, but copy$default exposes its receiver and mask arguments. |
| `toString` | ? Not reviewed line by line. |

### Findings

- The full Metadata annotation appears in the text; it carries the visible Kotlin class identity. Recorded forms: `@Metadata(d1 = …, d2 = …, …) runtime`.
- copy$default exposes receiver, mask and marker arguments, making the output read like lowered DEX. Recorded forms: `copy$default(Case, String, int, int, Object)`.
- append<(char)>(58) displays a character as its numeric value rather than a character literal. Recorded forms: `complex-annotation`.
- Property access remains getName()/getCount(); the val declaration form is not recovered.

### Ranked fixes

Improve character, copy$default, Metadata and property presentation. Recorded forms: `@Metadata`.

## Rounds

- Scalar-use disclosure: reread describe/toString colon and closing-parenthesis units; other member text and guard behavior remain unchanged.
- R1 2026-11-08: The full Metadata annotation appears in the text; it carries the visible Kotlin class identity. Recorded forms: `@Metadata(d1 = …, d2 = …, …) runtime`.
