# Fixture: array-basic

array creation/read/write. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — round 1, 2026-11-08

### Semantics

The recorded LLM verdict compares reference and output; `?` marks unjudged behavior.
| method | semantics |
|---|---|
| `sized` | ✔ Values and ordering agree. |
| `filled` | ✔ Values and ordering agree. |
| `bytes` | ✔ Values and ordering agree. |
| `flags` | ✔ Values and ordering agree. |
| `read` | ✔ Values and ordering agree. |
| `write` | ✔ Values and ordering agree. |
| `length` | ✔ Values and ordering agree. |
| `nested` | ✔ Values and ordering agree. |

### Findings

- Creation, reads/writes, length and two-dimensional indexing agree. Recorded forms: `.length`, `matrix[row][column]`.
- Array literals and new int[n] are assigned to intermediate locals before return. Recorded forms: `filled`, `bytes`, `flags`.

### Ranked fixes

Improve direct return of array construction/literals and parameter naming.

## Rounds

- R1 2026-11-08: Creation, reads/writes, length and two-dimensional indexing agree. Recorded forms: `.length`, `matrix[row][column]`.
