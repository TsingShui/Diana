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

Engine checks use the crate tests and ordinary CLI requests. Build/lint success,
byte roundtrip and finite corpus checks have different evidence scopes; none alone
proves complete decompilation correctness. Production safety and module-owned
transform checks remain required.

## Regression workflow status

The only GitHub Actions workflow is
[release.yml](../../.github/workflows/release.yml). It runs on `diana-v*` tag
pushes or a manual dispatch naming an existing tag, builds the CLI for all
release targets and publishes after every build succeeds. Ordinary branch pushes
and pull requests do not trigger CI. Format, lint and test checks run locally
under the contribution and testing policies.

[EVOLUTION](EVOLUTION.md#primary-iteration) defines the primary improvement loop:
fixture discovery, owning-module repair, newly authored complexity challenges,
LLM path-by-path semantic review, and preservation of original/prior effects.
A repair of the discovery case alone does not complete the iteration.
[FIXTURES](FIXTURES.md) uses paired Java explanations and Smali inputs without
requiring a fixed Java/D8/R8 toolchain or exact assembler version. The crate tests and ordinary CLI requests supply the available products for
review.

Run applicable implemented checks through `cargo nextest run`; report their actual
scope and results. Written semantic judgments remain separate from executed behavioral
checks.
