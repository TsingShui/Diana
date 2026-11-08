# Fixture: class-anonymous

anonymous class capturing a local and a field. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — round 1, 2026-11-08

### Semantics

The recorded LLM verdict compares reference and output; `?` marks unjudged behavior.
| method | semantics |
|---|---|
| `task` | ✔ Capture of this and step agrees. |
| `currentBase` | ✔ |

### Findings

- task constructs `ClassAnonymous$1(this, p0)` with the right captures, then returns it through a temporary. Recorded forms: `task()`, `new ClassAnonymous$1(this, p0)`.
- The final parameter modifier is omitted; the recorded difference is notation, not behavior.
- The anonymous class is rendered separately; captures are visible in constructor arguments.

### Ranked fixes

Improve direct constructor returns and parameter naming; review capture fields and constructor arguments in the separate anonymous class. Recorded forms: `return new ...(...)`.

## Rounds

- R1 2026-11-08: task constructs `ClassAnonymous$1(this, p0)` with the right captures, then returns it through a temporary. Recorded forms: `task()`, `new ClassAnonymous$1(this, p0)`.
