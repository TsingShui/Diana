# Diana documentation

Diana is a native Rust APK/DEX decompiler. The analyzed input is its only source
of decompilation facts. Available readable code, original identities and material
uncertainty are the product goal; Java legality and compilability are not output
gates.

[PIPELINE_DESIGN](PIPELINE_DESIGN.md) remains the approved architecture for the
complete P0–P9 product chain. The active backend is `crates/diana-decomp/`, a fork
of dexdec that consumes P0/P1 facts from `diana-facts` and returns Java-like text.
The staged P2–P9 implementation is archived under `ref-old-impl/v2/` and is not
part of the active build.

| Document | Responsibility |
|---|---|
| [PIPELINE_DESIGN.md](PIPELINE_DESIGN.md) | Approved product chain, stage ownership, algorithms and failure contracts. |
| [phases.md](phases.md) | Active stage map and Engine method scheduling. |
| [spec/00-overview.md](spec/00-overview.md) | Product boundary, failure kinds and safety obligations. |
| [spec/00-class.md](spec/00-class.md) | P0 class facts and independent member coverage. |
| [spec/01-input.md](spec/01-input.md) | Shared readers, MethodSource and P1 disassembly facts. |
| [spec/02-flow.md](spec/02-flow.md) | P2 execution/completion products and translation validation. |
| [spec/03-ssa.md](spec/03-ssa.md) | P3 exact-slot states, pruned SSA, original position mapping and correspondence checks. |
| [spec/04-types.md](spec/04-types.md) | P4 value/type sources, per-use views, Phi adaptations and position-related initialization. |
| [spec/05-variables.md](spec/05-variables.md) | P5 storage bindings, original transfer copies, exception snapshots and local gaps. |
| [spec/06-regions.md](spec/06-regions.md) | P6 regions, concrete transfers, protection ownership and monitor scope plans. |
| [spec/07-java.md](spec/07-java.md) | P7 owned expressions, declarations, execution phases, selected controls and local gaps. |
| [spec/08-class.md](spec/08-class.md) | P8 directory-driven assembly, preparation/metadata expressions and scope-aware names. |
| [spec/09-rendering.md](spec/09-rendering.md) | P9 pure rendering, explicit phase notation, source spans and output failure locality. |
| [Engine README](../../crates/diana-decomp/README.md) | Fork provenance, input boundary and license. |
| [CLI README](../../apps/diana/README.md) | Application commands, selectors and publication behavior. |
| [GUI DESIGN](../../apps/diana-gui/DESIGN.md) | Host-level resource, identity, navigation, view and result contracts; not an Engine integration status claim. |
| [SOURCE_MAP.md](SOURCE_MAP.md) | Source attribution, reproducibility and evidence boundaries. |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Contribution, Rust and acceptance rules. |
| [FIXTURES.md](FIXTURES.md) | Java explanations, Smali inputs, assembly records and independent expectations. |
| [quality/README](../../quality/README.md) | Output-quality fixtures, their committed DEX and the written reviews. |
| [EVOLUTION.md](EVOLUTION.md) | Primary fixture-driven repair/complexity-challenge/review loop and supplementary test-side behavioral acceptance. |
| [WRITING.md](WRITING.md) | Documentation conventions. |

Archived implementations are not active stages or alternate recovery paths. Removed specifications and historical
execution records remain available in git history. The archived implementations in `ref-old-impl/v1/` and `ref-old-impl/v2/`
supply semantic/check reference material, not another build or method-analysis
route.

Engine checks use the crate tests and ordinary CLI requests. Build/lint success,
byte roundtrip and finite corpus checks have different evidence scopes; none alone
proves complete decompilation correctness. Production safety and stage-owned
transform checks remain required.

## Regression workflow status

[EVOLUTION](EVOLUTION.md#primary-iteration) defines the primary improvement loop:
fixture discovery, owning-stage repair, newly authored complexity challenges,
LLM path-by-path semantic review, and preservation of original/prior effects.
A repair of the discovery case alone does not complete the iteration.
[FIXTURES](FIXTURES.md) uses paired Java explanations and Smali inputs without
requiring a fixed Java/D8/R8 toolchain or exact assembler version. The crate tests and ordinary CLI requests supply the available products for
review. The old staged inspection shell is archived with the staged pipeline.

The final-output-plan executor and admitted regression suite are not implemented.
There is no automated final-plan behavioral acceptance command or regression CI
job to report as passing. This does not prevent the primary LLM-reviewed
iteration; its decision is not a substitute for a machine behavioral result.
The root testing policy now permits automated tests and CI test jobs. That
permission does not implement the checker or activate a nonexistent suite.
[Checker implementation](EVOLUTION.md#checker-implementation) defines the planned
Rust modules and their qualification. Plan behavior and P9 textual fidelity
remain separate evidence scopes.
