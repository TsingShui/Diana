# Fixture: class-enum-body

enum with constructor state, a method and a constant body. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — round 1, 2026-11-08

### Semantics

The recorded LLM verdict compares reference and output; `?` marks unjudged behavior.
| method | semantics |
|---|---|
| `weight` | ✔ |
| `scaled` | ✔ |

### Findings

- weight() and scaled() bodies agree.
- Enum constants expand into static final dex_flags<enum> fields and static-block construction rather than source-style constant declarations. Recorded forms: `FIRST(1), SECOND(2) { … }`, `public static final dex_flags<enum> … `, `new ClassEnumBody("FIRST", 0, 1)`.
- The constant-body class ClassEnumBody$1 is instantiated in the static block; its override is not visible in that class view. Recorded forms: `new`.
- DEX enum/synthetic flags and system annotation visibility appear in the main text. Recorded forms: `dex_flags<enum>`, `dex_flags<synthetic>`, `@… system`.

### Ranked fixes

Improve enum constant/body declarations, flag/visibility presentation and constructor parameter names. Recorded forms: `dex_flags<>`, `system`.

## Rounds

- R1 2026-11-08: weight() and scaled() bodies agree.
