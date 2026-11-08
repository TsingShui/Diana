# Fixture: class-inner-member

member inner class with a synthetic accessor to a private field. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — round 1, 2026-11-08

### Semantics

The recorded LLM verdict compares reference and output; `?` marks unjudged behavior.
| method | semantics |
|---|---|
| `inner` | ✔ Capture and construction agree. |
| `currentTotal` | ✔ |

### Findings

- Both outer-class methods agree; inner() returns through a temporary and currentTotal() adds this qualification. Recorded forms: `this.`.
- The inner class is rendered separately as ClassInnerMember$Inner; the outer view shows construction, not the synthetic accessor implementation. Recorded forms: `new Inner(...)`.
- The short method body is obscured mainly by intermediate locals.

### Ranked fixes

Improve direct returns and separately review the inner class and synthetic-accessor call relationships.

## Rounds

- R1 2026-11-08: Both outer-class methods agree; inner() returns through a temporary and currentTotal() adds this qualification. Recorded forms: `this.`.
