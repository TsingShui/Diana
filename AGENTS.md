# Working on Diana

Diana is a native Rust decompiler and static analysis tool for APK, DEX, JAR
and JVM class files. Read the relevant package instructions and owning code
before changing behavior.

## Product and safety

- Treat input bytes as untrusted. Check sizes, ranges and arithmetic before
  slicing or allocating. Bound resource use, preserve backpressure and disclose
  display limits. The CLI or host owns process termination.
- Use the analyzed input as the sole source of facts. Keep available Java-like
  output, original identities and explicit uncertainty. Preserve instruction,
  argument, effect, exception, lock and initialization order. Text and JSON
  deliver the same result. Java legality and compilability are outside the
  recovery contract.
- Contain failures to the smallest reliably identified scope. Preserve
  independent fragments and members, classified errors, and precise reasons
  for unavailability. A wider stop requires a concrete dependency or resource
  reason.
- Keep one analysis chain. Fix the cause in the owning module and update its
  producers and consumers together. Keep sealed boundaries and the input
  guarantees they enforce.
- Keep independent correspondence checks, symbolic replay and duplicate
  graph/dominance verification under `cfg(test)`. Ordinary requests retain
  input validation and construction checks and run without independent verifiers.
- Production uses native Rust without external runtime or process dependencies.
  The CLI owns files and ZIP archives. The engine borrows input and forbids
  unsafe code. The only Rust `unsafe` operation is the read-only mapping of the
  leased input in `apps/diana/src/analysis/input.rs`, behind a local allowance.

## Working and verifying

- Use the current checkout for routine work. Use separate worktrees, build
  outputs and mutable run artifacts when concurrent writers or overlapping
  local changes need isolation. Preserve existing work. Resolve conflicting
  semantics with the user before integration.
- Validate the affected behavior. Run Rust tests with `cargo nextest run` and
  select relevant packages and cases. Report failures and unrun checks
  accurately. Use independent expectations for regression tests.
- For substantive recovery or semantic changes, follow the
  [fixture-driven repair workflow](docs/diana/EVOLUTION.md). Follow
  [FIXTURES](docs/diana/FIXTURES.md) when preparing inputs.
- Keep private or unauthorized inputs and identifying corpus data out of
  commits. Use ignored `.cache/` storage. Consult
  `tools/audit/desensitize_words.txt` and run
  `python3 tools/audit/desensitize_check.py --range HEAD`.
- Commit, push, publish or tag only when requested. Obtain permission before
  withdrawing or committing unrelated changes. Agent commits use
  `[AUTOMATED] ` followed by a Conventional Commit subject.
- Write repository documentation in English. See [WRITING](docs/diana/WRITING.md)
  for evidence and documentation guidance.

See [CONTRIBUTING](docs/diana/CONTRIBUTING.md) for Rust conventions, scoped
checks and release commands. See [SOURCE_MAP](docs/diana/SOURCE_MAP.md) for
source attribution and reference use.
