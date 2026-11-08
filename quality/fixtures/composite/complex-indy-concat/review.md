# Fixture: complex-indy-concat

string concatenation through StringConcatFactory (invoke-custom). The Java source is the reference; the DEX is generated from it
with the fixture's `javac.args`/`d8.args` or the shared defaults (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/ComplexIndyConcat;`

## Current verdict — round 1, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `simple` | ✔ The explicit call site retains the bootstrap and arguments. |
| `many` | ? Not reviewed line by line for promotion of all six arguments. |
| `insideLoop` | ? Not reviewed line by line for the indy call inside the loop. |

### Findings

- invoke-custom prints explicit call-site metadata for StringConcatFactory.makeConcat followed by the original invocation and arguments.
- A single concatenation expands into metadata and an explicit call.
- Bootstrap method, prototype and argument count remain available; no replacement facts are invented.

### Ranked fixes

Improve StringConcatFactory expression presentation while keeping call-site metadata available.

## Rounds

- R1 2026-11-08: invoke-custom prints explicit call-site metadata for StringConcatFactory.makeConcat followed by the original invocation and arguments.
