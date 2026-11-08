# Fixture: class-init-order

field initializers, an instance block, static init and this(...). The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — round 1, 2026-11-08

### Semantics

The recorded LLM verdict compares reference and output; `?` marks unjudged behavior.
| method | semantics |
|---|---|
| `total` | ✔ |
| `label` | ✔ |

### Findings

- Static/instance initialization order agrees: second=2, then the instance block, then first=p0. Recorded forms: `second = 2`, `first = p0`.
- total() loads each field into a local before addition. Recorded forms: `int first = this.first; …`.
- The no-arg constructor retains this(1); the static final LABEL value is assigned through a temporary in the static block.

### Ranked fixes

Improve field-read composition, constant presentation and parameter names. Recorded forms: `static final`.

## Rounds

- R1 2026-11-08: Static/instance initialization order agrees: second=2, then the instance block, then first=p0. Recorded forms: `second = 2`, `first = p0`.
