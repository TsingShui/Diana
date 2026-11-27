# Changelog

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
