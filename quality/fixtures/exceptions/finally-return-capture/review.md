# Fixture: finally-return-capture

A finally overwrites the local or field used by a pending return and records the
new value. `FinallyReturnCapture.java` explains the behavior;
`FinallyReturnCapture.smali` is the assembly input. The prepared `classes.dex`
was assembled from the Smali and copied beside it.

- Provenance: authored for this repository; no third-party code or data.
- Original owner: `Lfixture/FinallyReturnCapture;`.
- Subjects: `Lfixture/FinallyReturnCapture;->localReturn(I)I` and
  `Lfixture/FinallyReturnCapture;->fieldReturn(I)I`.
- R1 review base: repository HEAD `6d663d9`; engine sources unchanged from
  `12de370`.
- Expected behavior: the scenario table in `README.md`.

## Current verdict — round 2

Manually reread both Java/Smali methods and fresh P6/P9 delivery at `09e322b`,
plus localReturn's P2/P5 products. localReturn saves 1 or 2 before record(3); its body throw records
then rethrows. fieldReturn saves the field value 10 before writing 99 and
recording 99; its body throw still writes and records 99. A record failure
replaces the pending return or exception. No semantic mismatch was found under
the README's helper model.

localReturn now initializes its shared local to 1 before the mode-1 branch,
matching the original constant definition. The changed text preserves the
reviewed paths. The field-return local still shadows the field name, although
field accesses are qualified; this is a naming/readability concern.

R1's claim of a product-fact loss for the folded assignment is withdrawn.
P2 retains the two constant-3 definitions at PCs 17 and 23, and P5 records
`Materialization::Literal` for both. Printing record(3) folds those pure local
definitions; it does not lose the original instruction facts or change an effect.
Fresh delivery has no stopped member or rendering issue. Behavioral execution
remains unrun.

## Prior review — round 1

### Semantics

This is a manual source/output reading judgment, not an automated execution result.

| Method | Semantics |
| --- | --- |
| `localReturn` | ~ Equivalent behavior: the saved pre-finally value is returned and the finally's observable record call runs on both paths. The dead local assignment `result = 3` is folded into `record(3)`, so the finally body is not printed as a separate local write. |
| `fieldReturn` | ~ Equivalent behavior: the field is read before the overwrite, the returned local keeps the old value, and the field write and record use the new value. The generated local is named `state` and shadows the field. |

### Path comparison

The output has no `try`/`finally` text. `localReturn` prints the return value in
`v3`, then `record(3)` on the normal path and again in `H9`; `fieldReturn` prints
`int state = FinallyReturnCapture.state;`, then the field write, then
`record(99)`, then `return state`.

| Path | Reference and recovered behavior |
|---|---|
| `localReturn(0)` | The output selects `v3 = 1`, calls `record(3)`, and returns `v3`. The return value is 1, not the finally's overwritten 3. |
| `localReturn(1)` | `v3 = 2`, `record(3)`, return 2. |
| `localReturn(2)` | The constructed `ISE` is thrown; `H9` calls `record(3)` and rethrows. The return value does not exist on this path. |
| `localReturn` record failure | `record(3)` is outside any protected range; its failure replaces the pending return or body exception and propagates. |
| `fieldReturn(0)` | `state = 10`, the field read is saved to the printed local `state`, the field is written to 99, `record(99)` runs, and the saved value 10 is returned. |
| `fieldReturn(1)` | The constructed `ISE` is thrown; `H7` writes 99, records 99, and rethrows. |
| `fieldReturn` record failure | The record call follows the field write and has no handler; it replaces the pending return or exception. |
| Field ordering | The output reads the field before writing 99 and records 99 after the write; the source's finally has the same order. |

The pure local constant definition in `localReturn` is folded in the text: the source's finally assigns `result = 3` before calling `record(result)`,
while the output directly prints `record(3)`. The new local value is used only
as that known call argument and is not read after the call. Folding it therefore
changes no observable result, call argument, exception or heap write. It does mean the printed body no longer shows the finally
assignment itself.

### Concrete mismatch

None found in reviewed observable behavior. The returned values, field writes,
record arguments and exception identity replacement agree with the source. The
folded local constant remains in the original facts; it is not an observed
behavioral divergence or loss of the underlying product facts.

### Reading obstruction

The method has no `try`/`finally` container. The normal path is a pair of
`if`/`else` blocks and the handler is an `H7` or `H9` label reached through an
`exception_chain` line. In `localReturn`, the finally's local assignment is gone,
so a reader cannot see why `record(3)` belongs to a finally at all. In
`fieldReturn`, the generated local is named `state`, shadowing the field; the
declaration `int state = FinallyReturnCapture.state;` is required to read the
return correctly, but a skimming reader can misread `return state` as returning
the field after it was set to 99. This is a concrete readability hazard; the
saved-value and record ordering are preserved.

### Uncertain and unreviewed paths

- `record` is native; its argument and its failure are scenario inputs.
- The local assignment's omission is reviewed as unobservable because the local
  is dead after the return; this relies on the source not reading it elsewhere.
- No automated behavioral execution or Rust fixture test was run.

### Commands

```sh
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/finally-return-capture/classes.dex \
  --exec '{"cmd":"p9-all","text_only":true}'
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/finally-return-capture/classes.dex \
  --exec '{"cmd":"p6-all","details":true}'
```

## Rounds

- R1: Prepared `classes.dex` from the authored Smali; manual path comparison finds
  the saved return values, field writes and record ordering consistent with the
  source. The folded local constant and shadowing local name were recorded as
  readability findings; R2 corrects the original product-fact-loss assessment.
