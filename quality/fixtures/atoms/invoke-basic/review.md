# Fixture: invoke-basic

static/virtual/interface/constructor calls. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — round 1, 2026-11-08

### Semantics

The recorded LLM verdict compares reference and output; `?` marks unjudged behavior.
| method | semantics |
|---|---|
| `staticCall` | ✔ |
| `virtualCall` | ✔ |
| `interfaceCall` | ✔ |
| `constructorCall` | ✔ Construction agrees; the result is returned through a temporary. |
| `manyArguments` | ✔ |
| `listCall` | ✔ Generic arguments are missing from the presentation, without a recorded behavior mismatch. |

### Findings

- All four invocation forms preserve the referenced calls.
- Constructor results use an extra local; List<Integer> loses its generic argument.

### Ranked fixes

Improve direct constructor returns and generic-argument presentation.

## Rounds

- R1 2026-11-08: All four invocation forms preserve the referenced calls.
