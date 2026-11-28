# Changelog

## 0.4.23 - 2026-11-29

- Recovered annotation element defaults from DEX and JVM input.
- Localized malformed defaults and retained unaffected annotation members.

## 0.4.22 - 2026-11-28

- Preserved noncanonical NaN bits in recovered DEX and JVM expressions.
- Displayed original NaN bits in encoded field and annotation values,
  including early JVM ConstantValue field initialization.

## 0.4.21 - 2026-11-28

- Preserved selections between positive and negative zero in recovered DEX
  and JVM expressions, including call arguments and field and array writes.

## 0.4.20 - 2026-11-28

- Preserved unordered floating-point comparisons when recovering guard
  branches and short-circuit conditions in DEX and JVM methods.
- Preserved floating-point self-comparisons when an operand can be NaN.

## 0.4.19 - 2026-11-28

- Preserved single receiver evaluation, null checks and whole-range bounds
  checks before writes in DEX `fill-array-data` recovery.
- Retained floating-point payload bit patterns and compact array initializers
  when a fresh array has a proven sufficient length.

## 0.4.18 - 2026-11-28

- Preserved the referenced owner of DEX and JVM instance field reads and
  writes when a subclass declares a field with the same name.
- Recognized generic constructor calls in DecBench invocation checks.

## 0.4.17 - 2026-11-28

- Preserved active reads of static final fields with implicit default values.
  Cross-class reads still trigger initialization and observe its effects
  and failures in DEX and JVM recovery.

## 0.4.16 - 2026-11-28

- Preserved eager, left-to-right evaluation of effectful floating-point
  comparison operands in DEX and JVM recovery, including comparisons whose
  operands can throw.
- Kept the original NaN bias and signed-zero behavior when recovering
  floating-point comparisons.

## 0.4.15 - 2026-11-28

- Preserved explicit monitor scopes in DEX and JVM methods, including nested
  acquisitions of the same object and explicit scopes inside JVM synchronized
  methods.
- Used the original DEX monitor instructions to recover synchronization in
  bytecode methods carrying the declared-synchronized access flag.

## 0.4.14 - 2026-11-28

- Preserved runtime writes to static final primitive and string fields so
  cross-class reads still trigger initialization and observe its failures.
- Kept static initializer writes associated with their original declaring
  class, including when two classes share a simple name.

## 0.4.13 - 2026-11-28

- Preserved string reference identity when DEX builder chains and JVM
  runtime concatenations could otherwise become interned Java constant
  expressions. Literal-only builder chains keep their original calls.

## 0.4.12 - 2026-11-28

- Preserved `StringBuilder.append(Object)` and `StringBuffer.append(Object)`
  calls so object-to-string conversion stays in its original order relative
  to argument computations, including when either operation throws.

## 0.4.11 - 2026-11-28

- Preserved explicit capacities and their computation when recovering
  `StringBuilder` and `StringBuffer` chains from DEX and JVM bytecode.
  A capacity failure can still prevent later append arguments from running.
- Added DecBench coverage for negative capacities, returned strings,
  producer exception identity, and dispatch to competing typed catches.

## 0.4.10 - 2026-11-28

- Preserved effectful DEX element computations before `filled-new-array`.
  Arrays originally allocated before element stores can still recover as
  compact initializers.
- Added DecBench coverage for array allocation failures, element producer
  faults, typed catches, returned values, and shared references.

## 0.4.9 - 2026-11-28

- Preserved the DEX order of allocation, argument evaluation, initialization,
  and publication when each phase can expose different state to a catch.
  Recovered `new` expressions still combine argument operations when their
  original order, exception dispatch, and handler inputs agree.
- Added DecBench scenarios that exercise class initialization, allocation
  and constructor operands, catch state, and warm and erroneous-class retries.
  The source-aligned checker recognizes explicit initializer targets with
  array parameters and verifies their exact DEX member identities.

## 0.4.8 - 2026-11-28

- Kept effectful DEX argument computations before `new` when allocation
  and initialization have no intervening observable operations. This
  preserves the order of `Pattern.compile` and visitor construction in
  FairEmail's `HtmlHelper.autoLink`.

## 0.4.7 - 2026-11-28

- Preserved integral overloads when DEX and JVM recovery publish Java-like
  calls, `new` expressions, and constructor delegation. A `char` or `byte`
  keeps its value when an original invocation needs the wider formal.
- Added DecBench coverage for crossed argument types, numeric boundaries,
  and constructor binding, plus a JVM check of results and effect order.

## 0.4.6 - 2026-11-28

- Preserved the values an enclosing DEX catch sees when both normal and
  exceptional cleanup reach it through a shared `finally`.
- Added DecBench coverage for faults before and after a state update, two
  independent wide values, and repeated cleanup in a loop.

## 0.4.5 - 2026-11-28

- Preserved the local state at explicit DEX rethrows after cleanup. In
  FairEmail's `monitorAccount`, early scheduling failures retain the
  previous timestamp, and failures after a refresh retain the fresh value.
- Added DecBench coverage for this state transfer through split catcher
  entries, wide register moves, and cancellation failures that replace a
  pending exception.

## 0.4.4 - 2026-11-28

- Recovered a single DEX `finally` when split catch-all entries forward
  to shared cleanup and the normal path runs equivalent cleanup. This fixes
  cases where recovery could skip cleanup or swallow the pending exception.
- Preserved nested typed handlers and values observed by the enclosing catch.
  A cleanup failure still replaces the pending exception.
- Added DecBench fixtures for split exception entries, nested typed catches,
  and state carried into an enclosing catch.

## 0.4.3 - 2026-11-28

### DEX recovery

- Preserved effect order when merging guarded values. This fixes reordered
  database and network calls and premature adapter updates found in AntennaPod.
- Improved exception recovery for large methods and shared cleanup. Recovery
  preserves each path's arguments, exception dispatch, and cleanup order.
  Available method bodies remain visible when some exception dispatch is
  unresolved, with a diagnostic identifying the affected DEX edges.
- Recovered more short-circuit expressions while preserving their operand
  order and evaluation conditions.

### DecBench

- Added FairEmail to the reproducible public APK corpus and added full APK
  export evaluation with a per-method browser comparison for review.

## 0.4.2 - 2026-11-28

- Expanded the embedded Agent Skill with recommended workflows for APK, DEX,
  and JVM analysis. The guide includes original class and method coordinates,
  caller and callee queries, archive inspection, comparison, and export.
- Updated the workspace and release builds to Rust 1.100.

## 0.4.1 - 2026-11-28

- Added native analysis and Java-like source recovery for JAR archives and
  standalone JVM class files, with bytecode, cross-reference and outline queries.
- Added JVM archive comparison and package-organized export of JVM sources
  and resources. This release also includes the DEX comparison and recovery
  changes listed under 0.4.0.
- Restored compatibility with Rust 1.93 for release builds.

## 0.4.0 - 2026-11-27

### DEX comparison

- Added `diana diff OLD NEW` for comparing root DEX methods across APKs or
  standalone DEX files. Text and JSON include original method locators,
  normalized instruction changes, pairing evidence and unresolved matches.

### Export and decompilation

- Reduced CPU time and peak memory during bulk export by reusing control-flow,
  reaching-definition, predicate and Java type facts across analysis stages.
- Improved branch and exception recovery by preserving the inputs of
  predicates reevaluated after a guarded body and the order of cleanup effects.

## 0.3.0 - 2026-11-25

### Export

- Added `diana export INPUT -o DIRECTORY [--jobs N]` for package-organized
  Java-like sources from standalone DEX files and APK root images.
- APK export retains resources, assets and native libraries and decodes the
  manifest. Local recovery issues and relocated paths appear in the export
  report.

For methods with multiple exception regions, cleanup recovery now shares
immutable SSA origin and copy information across region comparisons.

### Decompilation

- Cleanup matching now distinguishes the argument values passed on normal and
  exceptional paths when reconstructing a finally block.
- Nested catch scopes respect the original protected instructions in typed
  handlers. A failure during an unprotected close escapes without retrying it.
- Value folding and continuation inlining keep potentially throwing operations
  under their original exception handlers when combining expressions or exits.

### CI

- Removed the separate `diana-cli` format, lint and build workflow. GitHub
  Actions now runs only the release workflow, triggered by `diana-v*` tags or
  manual dispatch with an existing tag.

## 0.2.8 - 2026-11-15

### Documentation

- Command help and the READMEs now state the analyzed image set: `classes`,
  `fields`, `methods`, `strings` and `xref` read root DEX images only, meaning
  `classes.dex` and `classes<N>.dex` at the archive root, or the whole input for
  a bare DEX. DEX stored in `assets/`, `res/raw/` or a nested archive is not
  scanned; the help names `entries` and `extract` as the way to analyze it.
- Removed the inaccurate "every DEX in an APK" scope claim from `classes`, `xref`
  and the top-level help notes.
- `strings` documents one row per original `string_ids` entry per root, so equal
  values are not merged, and that text output escapes control characters.
- List commands document the 50-row default with its truncation notice, the
  several-MB size of `--no-limit` output on a large APK, and `--filter` or `-o`
  as the intended way to read a large result.
- Added a `strings` example to the top-level help.

## 0.2.7 - 2026-11-14

### Agent Skill

- Added `diana skill` to print an embedded, minimal `SKILL.md` for Android
  reverse engineering. The skill directs agents to the CLI help for commands
  and detailed usage.
- Added `diana skill -i` (`--install`) to install the skill to
  `~/.agents/skills/diana/SKILL.md`, creating missing directories and reporting
  the installed path on stderr.
- Installation publishes the complete file atomically and refuses to overwrite
  an existing file, symlink or directory. The home directory comes from `HOME`
  on Unix or `USERPROFILE` on Windows and must be an absolute path.

### CI

- Removed the Linux stable full-workspace job, including the GUI build.
  Rust 1.93 CLI and engine gates remain on Linux, macOS and Windows.

## 0.2.6 - 2026-11-11

### Decompilation

- Restored original encoded static field values instead of silently rendering
  descriptor defaults. Scalar widths, negative zero and UTF-16 string units are
  preserved.
- Allocation and direct constructor calls with different owners now remain
  separate `allocate-instance` and `init-direct` operations, preserving the
  original constructor target, execution sites and surrounding effects.
- Accepted DEX map kind `0xF000` as opaque platform hidden-API metadata under
  the existing map envelope checks, allowing affected framework DEX images to
  reach normal analysis.
- Unavailable method bodies now contain explicit Diana warning comments with
  the recovery reason and issue URL. The diagnostic no longer invents an
  executable exception throw; original input throws remain readable code.

### CLI compatibility

- `get-class` and `get-method` JSON publications now use schema 5 and include
  structured method-local `issues` with the original identity, stage and reason.
- Method-local recovery failures publish the available result, report a
  diagnostic on stderr and return exit code 1 in both text and JSON formats.
  Output selected with `-o` remains available after a recovery failure.

### Documentation

- Retired obsolete pipeline specifications and references in the active
  contribution and package documentation.

## 0.2.5 - 2026-11-11

### Performance

- Cross-reference scans now use target-index prefilters before resolving
  annotations, static values, field types, and method return types.
- Field and method annotation association avoids resolving complete member
  declarations when the query only needs the declaring member identity.
- Type queries can answer return-type and field-type relations without
  materializing unrelated declarations.
- Code-reference batches reuse resolved targets and method contexts while
  preserving the first matching use in each method.

### Instrumentation

- Added an opt-in `perf` feature with fastrace spans and aggregated xref,
  annotation, class-data, batch, resolve, analyze, and publish counters.
  `app_profile` is the development entry point; normal release builds are
  unchanged.
- Removed the unused no-op `profile_scope!` compatibility macro and
  `crates/diana-decomp/src/profiling.rs`.
