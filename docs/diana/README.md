# Diana documentation

Diana is a native Rust APK/DEX decompiler. The analyzed input is its only source
of decompilation facts. Available readable code, original identities and material
uncertainty are the product goal; Java legality and compilability are not output
gates.

The active backend is `crates/diana-decomp/`, a fork of dexdec that owns the DEX
reader, class and decoded-method facts, and recovery to Java-like text. Its public
entry is `Engine` in `src/engine.rs`. Consult the implementation and package
READMEs for current interfaces and behavior.

| Document | Responsibility |
|---|---|
| [Engine README](../../crates/diana-decomp/README.md) | Fork provenance, input boundary and license. |
| [CLI README](../../apps/diana/README.md) | Application commands, selectors and publication behavior. |
| [GUI DESIGN](../../apps/diana-gui/DESIGN.md) | Host-level resource, identity, navigation, view and result contracts; not an Engine integration status claim. |
| [SOURCE_MAP.md](SOURCE_MAP.md) | Source attribution, reproducibility and evidence boundaries. |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Contribution, Rust and acceptance rules. |
| [FIXTURES.md](FIXTURES.md) | Java explanations, Smali inputs, assembly records and independent expectations. |
| [quality/README](../../quality/README.md) | Output-quality fixtures, their committed DEX and the written reviews. |
| [EVOLUTION.md](EVOLUTION.md) | Primary fixture-driven repair/complexity-challenge/review loop and supplementary test-side behavioral acceptance. |
| [WRITING.md](WRITING.md) | Documentation conventions. |

## Historical pipeline material

`PIPELINE_DESIGN.md`, `UNIFIED_RECOVERY_IR_DESIGN.md` and `phases.md`
describe the retired numbered-stage pipeline. They are retained for historical
reference and do not define current architecture, interfaces, migration authority
or acceptance requirements. Their commands and implementation-status claims may
refer to archived code. The old `spec/` chapters have been removed; their content
remains available in Git history. The archived implementations in `ref-old-impl/v1/` and
`ref-old-impl/v2/` are outside the active build.

Engine checks use the crate tests and ordinary CLI requests. Build/lint success,
byte roundtrip and finite corpus checks have different evidence scopes; none alone
proves complete decompilation correctness. Production safety and module-owned
transform checks remain required.

## Regression workflow status

[EVOLUTION](EVOLUTION.md#primary-iteration) defines the primary improvement loop:
fixture discovery, owning-module repair, newly authored complexity challenges,
LLM path-by-path semantic review, and preservation of original/prior effects.
A repair of the discovery case alone does not complete the iteration.
[FIXTURES](FIXTURES.md) uses paired Java explanations and Smali inputs without
requiring a fixed Java/D8/R8 toolchain or exact assembler version. The crate tests and ordinary CLI requests supply the available products for
review. The old staged inspection shell is archived with the staged pipeline.

Run applicable implemented checks through `cargo nextest run`; report their actual
scope and results. The supplementary final-plan executor design in EVOLUTION
refers to archived pipeline products and does not supply a current acceptance
command. Written semantic judgments remain separate from executed behavioral
checks.
