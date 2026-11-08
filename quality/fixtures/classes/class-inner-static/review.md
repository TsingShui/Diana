# Fixture: class-inner-static

static nested class with its own state. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — round 1, 2026-11-08

### Semantics

The recorded LLM verdict compares reference and output; `?` marks unjudged behavior.
| method | semantics |
|---|---|
| `holder` | ✔ |
| `useHolder` | ✔ |

### Findings

- holder() returns construction through a temporary; useHolder() adds qualification.
- The static nested class is rendered separately; outer references retain its original class name.

### Ranked fixes

Improve direct construction returns and parameter names.

## Rounds

- R1 2026-11-08: holder() returns construction through a temporary; useHolder() adds qualification.
