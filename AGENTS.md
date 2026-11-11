# Working on Diana

Diana is a native Rust APK/DEX decompiler. The workspace contains the CLI in
`apps/diana/` and the independent engine in `crates/diana-decomp/`. Read the
package-local `AGENTS.md`, where present, before working in either package.

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
- Never bypass a missing producer or sealed boundary with a parallel analysis
  route. Identify actual producer/consumer needs before changing an interface.

## Error handling

- All error paths MUST contain failure to the smallest reliably identified scope
  and propagate missing facts only to their actual dependents. Partial method
  recovery is a normal result.
- Analysis results MUST describe coverage, established facts and local
  unavailability;
  a downstream stage MUST NOT collapse mixed input into whole-method failure merely
  because one item is unavailable. Keep owning-stage construction and one method chain.
- Failure kind MUST NOT alone determine stop scope. A method-private invariant or
  allocation failure MUST NOT automatically terminate sibling methods, classes or
  DEX images; widening a stop requires a concrete shared-input/resource reason.
- Preserve recovered code, original facts, identities and precise unavailable
  ranges/reasons. Do not erase effects, fabricate values or promote incomplete
  analysis snapshots to complete products to appear tolerant.

## Workflow

- Contributors MUST develop and verify changes in a dedicated Git worktree and
  branch, not directly in the `main` working tree. Concurrent agents MUST use
  separate worktrees, build outputs and mutable run artifacts. Merge completed,
  checked commits into `main`; preserve concurrent commits and uncommitted work.
  Do not overwrite another agent's changes to resolve a conflict. Conflicting
  semantics require an explicit decision before integration.
- Contributors MUST NOT develop by accumulating local patches that mask symptoms,
  special-case observed inputs or bypass missing producer facts. Contributors MUST
  trace each problem to its architectural cause and implement a general solution
  in the owning module, with explicit input guarantees and failure behavior.
  Contributors MUST update affected producers, consumers and invariants together
  and remove superseded paths. Small, focused changes remain valid when they fix
  the shared cause; this rule does not require speculative frameworks or broad rewrites.
- Read the owning implementation and package documentation before capability
  changes. Identify consumed facts, guarantees, invariants and classified failure
  behavior.
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
  Structural checks and executed test results are supporting evidence, not
  substitutes for that review. Contributors MUST NOT generate or include numeric
  structure, naming or token-similarity quality ratings in fixture tooling or
  review records. Written judgments do not establish executed
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

## Documentation map

- [SOURCE_MAP](docs/diana/SOURCE_MAP.md): sources, attribution and reference use.
- [CONTRIBUTING](docs/diana/CONTRIBUTING.md): detailed contribution rules.
- [FIXTURES](docs/diana/FIXTURES.md): Java/Smali pairs and DEX reproduction evidence.
- [EVOLUTION](docs/diana/EVOLUTION.md): discovery-to-repair workflow and test-side acceptance design.
- [WRITING](docs/diana/WRITING.md): documentation conventions.
