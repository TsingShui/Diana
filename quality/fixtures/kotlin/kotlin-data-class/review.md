# Fixture: kotlin-data-class

data class and the members kotlinc generates (componentN, copy, copy$default, equals/hashCode/toString). The Kotlin source is the semantic reference for the LLM review below.

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

- R1 2026-11-08: The full Metadata annotation appears in the text; it carries the visible Kotlin class identity. Recorded forms: `@Metadata(d1 = …, d2 = …, …) runtime`.
