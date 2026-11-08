# Working on Diana

Diana is a native Rust APK/DEX decompiler. The workspace contains the CLI in
`apps/diana/` and the independent engine in `crates/diana-decomp/`. Read the
package-local `AGENTS.md` before working in either package.

## Product and safety

- Default retrieval MUST expose available readable code, preserve original member
  identities and DEX facts, and mark material uncertainty. A local failure MUST
  NOT suppress independently recoverable fragments within the same method or
  other members; text and JSON present the same result.
- Java legality or compilability is not a decompilation goal or output gate.
  Keep concrete recovery diagnostics, not whole-result verification badges;
  retain input safety and necessary construction error handling. Independent
  translation/correspondence verification belongs to tests, not normal requests.
- Normal decompilation MUST NOT run independent output validation, symbolic replay
  or duplicate graph/dominance verification. Compile these only under `cfg(test)`;
  do not add a runtime or feature switch that enables them on ordinary requests.
- Treat APK/DEX bytes as untrusted: check sizes, ranges and arithmetic before
  slicing or allocating. Do not panic, hang or allocate without bounds.
- The analysed input is the single source of decompilation facts. Do not require
  or parse external declarations, class files, android.jar, classpaths, symbol
  databases or network facts; treat absent facts as Unknown and keep the readable
  result with explicit uncertainty.
- Preserve effect, instruction, argument, exception, lock and initialization
  order, classified errors and backpressure. Bounded queues are not pagination;
  never silently truncate a successful result. Request cancellation callbacks and
  tokens are not part of the CLI-oriented engine API; process termination belongs
  to the CLI/host.
- Production is native Rust and has no external runtime or process dependency.
  The single permitted `unsafe` operation is the read-only memory mapping of the
  leased input file in `apps/diana/src/analysis/input.rs`, behind one
  `#[allow(unsafe_code)]`; the engine keeps `unsafe_code = "forbid"`. The
  application owns files and ZIPs; the engine borrows input bytes and declaration
  facts.
- Use one shared ordinary-method analysis chain. Migrate only to the approved
  target below; rendering consumes existing products and original facts without
  starting analysis or retrying reconstruction.
- Never bypass a missing producer or sealed boundary with a parallel analysis
  route. Follow the approved refactoring authority below for interface changes.

## Error handling

- All error paths MUST follow [failure locality](docs/diana/PIPELINE_DESIGN.md#9-local-failure-and-method-exit):
  contain failure to the smallest reliably identified scope and propagate missing
  facts only to their actual dependents. Partial method recovery is a normal result.
- P1–P9 MUST each implement the [local-result contract](docs/diana/PIPELINE_DESIGN.md#92-p1p9-uniformly-consume-local-results).
  Published products describe coverage, established facts and local unavailability;
  a downstream stage MUST NOT collapse mixed input into whole-method failure merely
  because one item is unavailable. Keep owning-stage construction and one method chain.
- Failure kind MUST NOT alone determine stop scope. A method-private invariant or
  allocation failure MUST NOT automatically terminate sibling methods, classes or
  DEX images; widening a stop requires a concrete shared-input/resource reason.
- Preserve recovered code, original facts, identities and precise unavailable
  ranges/reasons. Do not erase effects, fabricate values or promote incomplete
  analysis snapshots to complete products to appear tolerant.

## Pipeline refactoring authority

The target in [PIPELINE_DESIGN](docs/diana/PIPELINE_DESIGN.md) is approved: P0 owns
class facts; an independent method reader prepares input; P1 disassembles; P2–P7
process one method; P8 assembles the class and plans names; P9 renders. Engine
holds and delivers products and schedules method-level tasks. Current phase
chapters describe the existing implementation until migrated to this target.

- Contributors MAY decide technical details within this target without repeated
  approval: private product layout, lifetimes, algorithm invariants, classified
  outcomes, check organization and implementation order. Prefer implementing the
  new product structure; existing algorithms can be reused or rewritten without
  preserving old private IR layouts or moving files one-to-one. Contributors MUST
  document producer/consumer needs, retain input safety and test-owned semantic
  verification obligations,
  and remove replaced paths when the new chain enters the shared formal entry.
  Rewriting MUST NOT introduce a second method-analysis or retry route.
- Contributors MUST NOT change agreed output behavior, hide available members or
  introduce a generated dispatcher. Natural and explicit Java-like expressions
  remain readable results, not whole-method qualification gates.
- Contributors MUST NOT introduce external declarations, new dependencies or
  numeric resource quotas for this refactoring.
- Contributors MUST NOT change phase ownership or scheduling beyond the approved
  target. Stage implementations MUST NOT invoke another stage's analysis.
- Contributors MUST NOT widen stage input access, exports or product-construction
  authority beyond the approved producer/consumer contracts. Private fact views
  and owning-stage constructors MUST NOT become general upstream access.
- Default approval does not authorize these prohibited changes. If the target
  cannot be implemented within these constraints, stop the affected work and
  report the concrete conflict; do not add a bypass or treat silence as an exception.

## Workflow

- Contributors MUST NOT develop by accumulating local patches that mask symptoms,
  special-case observed inputs or bypass missing producer facts. Contributors MUST
  trace each problem to its architectural cause and implement a general solution
  in the owning module, with explicit input guarantees and failure behavior.
  Contributors MUST update affected producers, consumers and invariants together
  and remove superseded paths. Small, focused changes remain valid when they fix
  the shared cause; this rule does not require speculative frameworks or broad rewrites.
- Read the owning [stage chapter](docs/diana/README.md) before capability
  changes.
  Identify consumed facts, guarantees, invariants and classified failure behavior.
  If contracts conflict with the change, stop and request a decision.
- The primary improvement process is the fixture-driven loop in
  [EVOLUTION](docs/diana/EVOLUTION.md): discover, repair the general cause,
  generate new more-complex same-mechanism fixtures, perform LLM semantic review,
  and recheck original/prior effects. Contributors MUST complete the complexity
  challenge and required regression checks before declaring an iteration effective;
  fixing the discovery case alone is insufficient. Unimplemented, unrun or
  unsupported checks MUST NOT be reported as passing.
- For detailed architecture, Rust conventions, dependency choices, acceptance,
  file verification, corpus handling and reference isolation, read
  [CONTRIBUTING](docs/diana/CONTRIBUTING.md) for the relevant work. Read
  [WRITING](docs/diana/WRITING.md) before editing Diana documentation.
- All repository documentation MUST be written exclusively in English, including
  READMEs, specifications, fixture reviews and AGENTS.md files. Contributors MUST
  NOT write documentation in Chinese or any other non-English language.
- For development verification of the engine, use the `dbg_shell` Cargo example
  as the single inspection entry:
  `cargo run -p diana-decomp --example dbg_shell --locked -- --dex <classes.dex> --exec '<json>'`.
  It accepts a bare DEX, emits one JSON response per command, and exposes the
  delivered P0/P1/P2/P3/P4/P5/P6/P7/P8/P9 products. `class` is the `class_def` index of that image and
  `method` is the engine-wide `MethodId`, not an index inside the class. Do not add phase-specific debug binaries, renderers or
  debug examples.
- Contributors MAY develop unit, integration, end-to-end and regression tests,
  fixture preparation tools and CI test jobs. Tests MUST use independent
  expectations and current product contracts. Fixture inputs and scenario
  expectations MUST stay in `quality/fixtures/<theme>/<slug>/`. Engine fixture
  checks MUST live in `crates/diana-decomp/tests/`, link to those inputs and use
  the ordinary Engine entry; shared test helpers belong in `tests/support/` only
  when actual cases need them. Keep test implementations out of production stage
  modules. Review historical cases before reuse; do not restore obsolete output
  gates or alternate analysis routes.
  Engine correspondence/replay checkers MUST remain under `cfg(test)`, consuming
  products delivered through the ordinary Engine entry without widening stage
  access or construction authority.
- Contributors MAY author paired Java explanations, Smali inputs and independent
  expectations for reproduction and regression under
  [FIXTURES](docs/diana/FIXTURES.md).
  All fixtures MUST live in `quality/fixtures/<theme>/<slug>/`, including
  reproduction and regression Java/Smali pairs. Reuse the matching theme; create
  it when absent. Do not create a top-level `fixtures/` or package-local fixture
  tree. Keep source explanations, assembly inputs, independent expectations and
  reviews beside the fixture. Assemble in `.cache/fixtures/<slug>/` without a
  per-fixture tool-version pin, record the actual environment, and publish the
  prepared `classes.dex` beside its sources so inspection/tests need no assembler.
  Only `quality/fixtures/**/classes.dex` is exempt from the generated-output ban;
  other generated files stay ignored. Written quality reviews support the primary
  iteration decision; they are judgements, not executed behavioral tests or
  machine regression acceptance.
  Test authorization does not permit independent validation on production requests.
- Fixture reviews MUST primarily use LLM semantic-equivalence review: compare
  the original input/source behavior with the current decompiled output path by
  path, including helper behavior, results, effect order and exception handling.
  Reviewers MUST record their reasoning and mark uncertain or unreviewed paths.
  Structural checks, similarity scores and test results are supporting evidence,
  not substitutes for that review. Written judgments do not establish executed
  behavioral checks or override failed tests and regression acceptance rules.
- Rust tests MUST run through `cargo nextest run`, not `cargo test`. Use package
  and test filters for the affected scope. Missing nextest is a setup failure;
  do not silently substitute another runner.
- Production input safety, analysis invariants and semantics-preserving transform
  checks remain required. Existing independent verifier code MUST NOT run in
  ordinary requests. Build/lint success does not establish semantic correctness.
- Respect uncommitted work; obtain approval before withdrawing or committing
  unrelated changes. Do not commit, push, publish or tag unless requested.
  Authorized commit subjects start with `[AUTOMATED] ` and a Conventional type.
- Keep private/unauthorized APKs and identifying corpus data out of commits;
  use `.cache/` locally. Consult `tools/audit/desensitize_words.txt` and run
  `python3 tools/audit/desensitize_check.py --range HEAD`.

## Common checks

Run from the repository root with Rust 1.93 or newer:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo build --workspace --release --locked
```

The CLI and engine hold the 1.93 minimum and CI gates them on that toolchain on
Linux, macOS and Windows. The GPUI desktop application follows current stable and
needs the platform GUI libraries; CI gates the full workspace on Linux stable only.

Run applicable format, lint, build and implemented test checks. Before integrating
an evolution repair, run its required regression set under EVOLUTION. Report
failures, missing cases and unrun evidence accurately; an empty test run or an
unimplemented checker is not acceptance evidence.

## Documentation map

- [phases](docs/diana/phases.md): phase map and current scheduling entry.
- [documentation entry](docs/diana/README.md): active contracts and package docs.
- [PIPELINE_DESIGN](docs/diana/PIPELINE_DESIGN.md): approved P0–P9 architecture.
- [spec](docs/diana/spec/): product boundary and implemented P0/input/P1/P2/P3 contracts.
- [SOURCE_MAP](docs/diana/SOURCE_MAP.md): sources, attribution and reference use.
- [CONTRIBUTING](docs/diana/CONTRIBUTING.md): detailed contribution rules.
- [FIXTURES](docs/diana/FIXTURES.md): Java/Smali pairs and DEX reproduction evidence.
- [EVOLUTION](docs/diana/EVOLUTION.md): discovery-to-repair workflow and test-side acceptance design.
- [WRITING](docs/diana/WRITING.md): documentation conventions.

PIPELINE_DESIGN is the approved architecture, not an implementation-status claim.
Use phases and package READMEs for active capabilities; archived specifications
and verification gates are not current contracts or acceptance evidence.
