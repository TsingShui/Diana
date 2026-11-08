# Fixture: typed-catch-shared-cleanup

Multi-catch and a broader typed catch sharing a finally with the normal path.
`TypedCatchSharedCleanup.java` explains the behavior;
`TypedCatchSharedCleanup.smali` is the assembly input. The prepared `classes.dex`
was assembled from the Smali and copied beside it.

- Provenance: authored for this repository; no third-party code or data.
- Original owner: `Lfixture/TypedCatchSharedCleanup;`.
- Subject: `Lfixture/TypedCatchSharedCleanup;->classify(Ljava/lang/String;I)I`.
- R1 review base: repository HEAD `6d663d9`; engine sources unchanged from
  `12de370`.
- Expected behavior: the scenario table in `README.md`.

## Current verdict — round 2

Manually reread the Java explanation, Smali and fresh P6/P9 delivery at
`09e322b`. The original clause order is NFE, NPE, RuntimeException, catch-all.
NFE/NPE enter the single H14 body and return 111 after closing detail then res;
the broad path returns -2; Error enters the outer cleanup. A failing detail
finally replaces a pending read failure, and outer-close failure can replace it
again. No semantic mismatch was found on these paths.

R1 did not catch inconsistent handle notation in the README and its own traces.
The authored helper model returns 50 from open(0) and 51 from open(1), so actual
close arguments are 50 and 51, not acquisition IDs 0 and 1. Those examples are
corrected from the helper model and Smali argument flow, independently of the
decompiled output.

The current text is unchanged from the inspected `6d663d9` engine. P6 delivers
only the nested detail-read try. In `crates/diana-decomp/src/p6/protection.rs`, natural-try planning
explicitly rejects a clause pattern with repeated handler targets. The first two
typed clauses share H14, so this is a concrete producer-side barrier, not merely
missing P9 syntax. Nested outer protection/shared cleanup introduces additional
ownership constraints; removing the rejection alone is not an established repair.
The explicit chain retains clause priority and effects. Fresh delivery has no
stopped member or rendering issue. Behavioral execution remains unrun.

## Prior review — round 1

### Semantics

This is a manual source/output reading judgment, not an automated execution result.

| Method | Semantics |
| --- | --- |
| `classify` | ~ Equivalent behavior with a different shape: catch priority, the -2/111 results, the nested detail close and the shared outer close agree, but the typed catch clauses are printed as labeled handler blocks and `exception_chain` routes. |

### Path comparison

The output keeps a natural nested `try`/`catch (all)` for the detail read, but the
outer typed protection is an explicit graph. The route line is

```text
exception_chain c1 { 0: NumberFormatException -> H14; 1: NullPointerException -> H14;
                     2: RuntimeException -> H20; 3: all -> H24; }
```

and the normal body ends with `v19 = v9; goto L22;` before the `H14`, `H20`,
`L22` and `H24` blocks.

| Path | Reference and recovered behavior |
|---|---|
| Normal parse | O(0), P("x"), then the shared close C(50) and return 7. The output prints `goto L22`, `close(v2)`, `return v19`. |
| Constructed NFE or NPE | The throw carries chain `c1`; clauses 0 and 1 both target `H14`. `H14` calls O(1), reads the detail, adds 100, closes the detail, then jumps to `L22` and closes the first resource. Result 111. |
| Parse throws NFE or NPE | The same `H14` path applies after `parse`, showing that the multi-catch target is shared. |
| Constructed ISE or caught `AE` | Clause 2 targets `H20`; `H20` sets the result to -2 and falls into `L22`. The first close still runs. |
| `Error` | The catch-all clause targets `H24`; `H24` closes the first resource and rethrows. |
| Detail read failure | The nested printed `catch (all)` captures the failure, calls C(51), and rethrows. The rethrow carries `c2` and enters `H24`, which closes the first resource. |
| Detail close failure | The normal detail close carries `c2`; a failure enters `H24`, closes the first resource and propagates the close failure. |
| Detail close failure while the read failure is pending | The detail close replaces the read failure on the explicit-finally path; the outer close then runs and may replace it again. No suppression is printed. |
| First-resource close failure | The shared `close(v2)` at `L22` has no chain marker, so it propagates after the normal or broad path and discards the pending return. |

Catch priority is preserved in the chain order: `NumberFormatException` and
`NullPointerException` are checked before `RuntimeException`, and the catch-all is
last. The multi-catch grouping is preserved by both clauses naming `H14`. The four
routes are not a semantic mismatch, but the printed form does not show a
`catch (NumberFormatException | NullPointerException)` or
`catch (RuntimeException)` clause.

### Concrete mismatch

None found in the reviewed paths. The typed routes, shared close and nested
detail cleanup agree with the source.

### Reading obstruction

The normal body jumps over `H14` and `H20` to a shared `L22` block. `H14` and
`H20` look like ordinary labeled blocks after the jump, not like catch clauses.
A reader must decode `exception_chain c1` to learn that `H14` is the
multi-catch target and `H20` is the `RuntimeException` target, and must remember
that chain order is catch priority. The outer finally is only visible as `H24`
and the `c2` catch-all route. The nested detail `try`/`catch` is readable, which
makes the contrast between the natural inner protection and the explicit outer
protection especially visible. This is a concrete reading obstruction; the
underlying execution facts are preserved. The triggering condition is not merely
a typed catch plus a finally: the typed handler body performs calls that must be
covered by the outer finally, so that handler region cannot be naturalized with
the outer catch-all.

The existing `quality/fixtures/composite/complex-exceptions` fixture shows the
same loss for the simpler `nestedFinally` method: its current output also prints
`exception_chain c2 { 0: ArithmeticException -> H9; 1: all -> H7; }` and a
separate `H9` block instead of a `catch (ArithmeticException)` clause. That raises
the obstruction above a single-fixture notation choice.

### Uncertain and unreviewed paths

- The helper bodies are native; `parse` returns or throws only according to the
  scenario model.
- `addSuppressed` is not used by this fixture; the explicit finally replaces
  rather than suppresses.
- Constructor failure and asynchronous exceptions are not modeled.
- No automated behavioral execution or Rust fixture test was run.

### Commands

```sh
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/typed-catch-shared-cleanup/classes.dex \
  --exec '{"cmd":"p9-all","text_only":true}'
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/typed-catch-shared-cleanup/classes.dex \
  --exec '{"cmd":"p6-all","details":true}'
```

## Rounds

- R1: Prepared `classes.dex` from the authored Smali; manual path comparison finds
  the priority, result selection and cleanup order consistent with the source.
  The typed catch clauses are not recovered as clauses; this is recorded as the
  fixture's main reading obstruction.
