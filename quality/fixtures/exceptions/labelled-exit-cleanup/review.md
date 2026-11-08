# Fixture: labelled-exit-cleanup

Try-with-resources in an outer loop with labelled continue, labelled break and a
return inside a nested loop. `LabelledExitCleanup.java` explains the behavior;
`LabelledExitCleanup.smali` is the assembly input. The prepared `classes.dex` was
assembled from the Smali and copied beside it.

- Provenance: authored for this repository; no third-party code or data.
- Original owner: `Lfixture/LabelledExitCleanup;`.
- Subject: `Lfixture/LabelledExitCleanup;->scan(II)I`.
- R1 review base: repository HEAD `6d663d9`; engine sources unchanged from
  `12de370`.
- Expected behavior: the scenario table in `README.md`.

## Current verdict — round 2

Manually reread the Java explanation, Smali and fresh P6/P9 delivery at
`09e322b`, using the README's native-helper models. Continue closes before the
row increment and outer continuation; break closes without incrementing the row;
return closes after saving the updated total. Read failures suppress a failing
close on the same primary. Acquisition and normal-exit close failures remain
outside that resource handler. No new semantic mismatch was found.

The two zero initializations now appear separately in their original definition
order. This changes no call, result or loop path. The remaining `goto L9` is a
readability limitation, not a demonstrated wrong transfer. Fresh delivery has no
stopped member or rendering issue. Behavioral execution remains unrun.

## Prior review — round 1

### Semantics

This is a manual source/output reading judgment, not an automated execution result.

| Method | Semantics |
| --- | --- |
| `scan` | ✔ Equivalent by path comparison: every inner-loop exit closes the current handle once, in the same order as the source, and exceptional close failures suppress on the primary. |

### Path comparison

The current output keeps the outer loop with an `R39:` label, the inner loop, and
one `try`/`catch (all)` around the read plus its control comparisons. The normal
close and each exit-specific close are outside that catch.

| Path | Reference and recovered behavior |
|---|---|
| Normal inner completion | Four reads for row 0, close handle 0, increment row, repeat for row 1, close handle 1, leave the outer loop and return the accumulated total. The output's `while (v6 < 4)` has the same four iterations and the same post-loop close and row increment. |
| Labelled continue | When the read equals `limit`, the output calls `close(v2)`, increments the row, and executes `continue R39`. `R39` is the outer loop, so the pending inner work and the remaining columns are skipped exactly as `continue outer` requires. |
| Labelled break | When the read is negative, the output calls `close(v2)` and executes `break R39`, leaving the outer loop without the row increment and returning the accumulated total. |
| Return | When the total exceeds 100, the output calls `close(v2)` and returns the accumulated total. The read value has already been added. |
| Read failure | The read is inside the catch. The handler captures the throwable, closes the handle, suppresses a close failure on the captured throwable, and rethrows. The exit closes are not in this protected range. |
| Normal close failure | A failing close on the normal inner completion, continue, break or return path is printed after the close that can throw and is not wrapped by the same catch. The failure therefore propagates and the pending transfer is discarded. |
| Acquisition failure in row 1 | The outer row-0 close has already run; `open(1)` is outside any handler and propagates without closing an unacquired handle. |

The output's `if (v3 != p1) { goto L9; }` is an inverted branch with a goto, and
`continue R39` is printed explicitly. These are shape differences; the branch
targets and the close placements agree with the source. The output also prints the
increment before `continue R39` because the source's outer `for` update runs on
that path; the outer `while` has no separate update section.

### Readability

The method is readable as nested loops with a natural inner `try`/`catch`; the
remaining friction is the explicit close before each exit, the `goto L9` used for
the continue-versus-body split, the `R39:` label, and the generated capture names.
No `exception_chain` line is needed. The absence of a compact `try (resource...)`
form is a shape limitation, not a semantic mismatch.

### Commands

```sh
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/labelled-exit-cleanup/classes.dex \
  --exec '{"cmd":"p9-all","text_only":true}'
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/labelled-exit-cleanup/classes.dex \
  --exec '{"cmd":"p6-all","details":true}'
```

No automated behavioral execution and no Rust fixture test were run.

## Rounds

- R1: Prepared `classes.dex` from the authored Smali; direct semantic reading
  finds the normal and exceptional loop exits consistent with the source. The
  remaining limits are explicit notation and the unrun behavioral executor.
