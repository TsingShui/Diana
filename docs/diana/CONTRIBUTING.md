# Contributing to Diana

Follow the [root instructions](../../AGENTS.md) and any package instructions
for the affected code. The [documentation index](README.md) links to current
product interfaces and workflow guides.

## Architecture

Before changing engine behavior, read the [engine README](../../crates/diana-decomp/README.md)
and the owning implementation. Identify the producer, consumer, input guarantees,
and failure and resource behavior. Preserve available code, original identities
and classified failures when moving an analysis boundary.

- Use the ordinary engine entries for recovery. Rendering consumes analysis
  products and original facts. It does not start another method analysis.
- Keep operation, definition/use, effect, storage and transfer facts shared
  across recovery representations. Initialization and array proofs use those
  facts in their owning modules. The raw exit delivers available original facts
  without retrying analysis or reconstructing values.
- Use format rules and local proofs or patterns in their owning modules.
  Avoid input-name special cases, whole-method templates, alternate recovery
  paths and generated dispatchers. Remove the replaced path when a migration
  is complete.
- The application owns files and archives. The engine borrows input bytes and
  declaration facts. Keep fallible allocation and checked ranges. Use
  `try_reserve` where the input controls allocation size and report failures
  explicitly. Use actual format and admission limits for resource bounds.
- Fuzzy queries may buffer and rank all matches. Other query modes retain
  completion order. Apply xref display limits after ranking and caller expansion.
  Preserve original values, match identities and local issues.

## Testing policy

Choose checks based on the affected behavior and packages. For a docs-only
change, inspect the diff, validate links and check changed commands or workflow
claims where applicable. For Rust changes, use the Rust version in the workspace
manifest. Here is an example for the CLI and engine:

```sh
cargo fmt -p diana -p diana-decomp -- --check
cargo clippy -p diana -p diana-decomp --all-targets --all-features --locked -- -D warnings
cargo build --locked -p diana -p diana-decomp
cargo nextest run --locked -p diana -p diana-decomp
```

Choose the relevant packages and test filters. For GUI changes, select
`diana-gui` and follow its [package instructions](../../apps/diana-gui/AGENTS.md)
for platform checks. Use workspace-wide checks when the change affects the
workspace broadly. Measure performance with release builds on representative,
authorized inputs. Real APK or ART validation requires explicit opt-in and
authorized reproducible inputs.

Use the ordinary production entry or owning-module interface in regression tests.
Use [FIXTURES](FIXTURES.md) for fixture preparation and [EVOLUTION](EVOLUTION.md)
for substantive decompilation changes. Keep test-only construction within the
same stage ownership boundaries. Report executed outcomes, blocked checks and
untested scenarios separately. Build and lint checks establish build and lint
status. [Current CI](README.md#regression-workflow-status) describes what runs
remotely.

## Rust and dependencies

- Follow rustfmt, the established Clippy baseline and the
  [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/). Keep lint
  exceptions local and explain the invariant.
- Use newtypes for confusable coordinates across module boundaries and enums
  for mutually exclusive states. Keep partial and complete products distinct.
  Only the owning stage constructs a sealed product.
- Preserve classified errors through core interfaces and add context at the
  application boundary. Use checked arithmetic and conversions for untrusted
  ranges and counts. Explain the local invariant behind any narrowing cast,
  `unwrap` or `expect` in production.
- Prefer concrete implementations, direct control flow and borrowed input.
  Introduce abstractions when they serve actual consumers or isolation. Keep
  `mod.rs` focused on wiring. Document API guarantees, invariants, units and
  failure behavior.
- Use `serde_json` and `serde` major version 1. Obtain approval for a different
  JSON library. Use `fastrace` 0.7 behind the opt-in `perf` feature for timeline
  instrumentation. The application owns reporter setup and flushing. Spans
  describe compute stages without input contents or sensitive identities.

## References and corpus data

Use pinned source URLs and review licenses before reusing ideas. Keep archived
implementations out of active builds, tests and dependencies.
`tools/fetch_references.py` downloads references into ignored `.cache/references/`.
Preserve local reference contents and changes. Keep offline paper or slide
references as UTF-8 text and record the original public URL.

### Corpus data and sensitive inputs

Follow the root corpus and audit rules. Keep provenance and scenario
expectations in [fixtures](FIXTURES.md). Prefer independently authored cases
with a documented source, license and recipe.

## File verification

Obtain explicit authorization before adding content-fingerprint verification
to the repository's reference download, fixture preparation or cache tooling.
Existing digests record provenance. Keep tool versions, dependency locks,
path/range/size checks and independent semantic evidence. Preserve
format-defined ZIP CRC and original DEX header checks.

## Contribution and releases

Retain the existing license files. Diana packages use `publish = false`.
The CLI's `diana-vMAJOR.MINOR.PATCH` tag must match its Cargo package version.
The [release workflow](../../.github/workflows/release.yml) builds the CLI for
Linux, macOS and Windows and publishes after every target succeeds. The engine
and GUI have independent versions.

### Build a profile-guided release CLI

Use a host that can run the target binary and has the target's native C tools
and linker. Install the workspace's Rust toolchain with LLVM tools. Run the
profile-guided build from a Git checkout:

```sh
rustup toolchain install 1.100.0 --profile minimal --component llvm-tools-preview
rustup run 1.100.0 python3 tools/release/build_cli.py
```

Pass `--target <triple>` to select a runnable target explicitly. The script
packages tracked DEX fixtures into a multidex APK and checks the instrumented
CLI's export against the fixture expectations. It merges the profiles and
rebuilds with them. The finished binary is
`target/release-pgo/<triple>/diana` or `diana.exe` on Windows.
