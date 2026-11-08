# Fixture: field-basic

static/instance field access. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — round 1, 2026-11-08

### Semantics

The recorded LLM verdict compares reference and output; `?` marks unjudged behavior.
| method | semantics |
|---|---|
| `readStatic` | ✔ Reads, writes and ordering agree. |
| `writeStatic` | ✔ Reads, writes and ordering agree. |
| `readLabel` | ✔ Reads, writes and ordering agree. |
| `readInstance` | ✔ Reads, writes and ordering agree. |
| `writeInstance` | ✔ Reads, writes and ordering agree. |
| `addToInstance` | ✔ Reads, writes and ordering agree. |

### Findings

- Reads/writes agree; extra this/class qualification and field-read locals make the text longer. Recorded forms: `this .`, `X . `, `int ID = this.ID`.
- `value += delta` expands into separate read, add and write operations.

### Ranked fixes

Improve compound-assignment presentation and unnecessary qualification. Recorded forms: `+=`, `this.`.

## Rounds

- R1 2026-11-08: Reads/writes agree; extra this/class qualification and field-read locals make the text longer. Recorded forms: `this .`, `X . `, `int ID = this.ID`.
