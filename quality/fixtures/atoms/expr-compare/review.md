# Fixture: expr-compare

comparisons and boolean composition. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — round 1, 2026-11-08

### Semantics

The recorded LLM verdict compares reference and output; `?` marks unjudged behavior.
| method | semantics |
|---|---|
| `lessThan` | ✔ |
| `atLeast` | ✔ Inverted but equivalent. |
| `same` | ✔ |
| `different` | ✔ |
| `both` | ✔ Inverted but equivalent. |
| `either` | ✔ |
| `negated` | ✔ |

### Findings

- Comparisons agree, but direct predicate returns become a local plus If. Recorded forms: `return a < b`, `int v; if (a < b) …`.
- Logical negation and conjunction also expand into locals and If. Recorded forms: `!value`, `a && b`.

### Ranked fixes

Improve direct predicate-return presentation.

## Rounds

- R1 2026-11-08: Comparisons agree, but direct predicate returns become a local plus If. Recorded forms: `return a < b`, `int v; if (a < b) …`.
