# Fixture: three-resources-late-failure

## Current verdict — scoped catch capture

Compared the same prepared input on `c95c6dd` and this repair. Reviewed header
bindings: `all v10`, `all v11`, `all v12`, `all v13`, `all v14`, `all v15`. The original captured object is bound once at exceptional
entry, before the unchanged receiving/body actions. Its exact-state uses remain
inside that handler and have no capture-phase, Phi or copy dependency requiring
an outer assignment. Normal execution does not bind or run the handler.

After accounting for that binding and its removed private declaration/assignment,
the complete remaining output is exact. Original P1/P3/P4/P5 products agree.
Thus the earlier independently reasoned normal, caught, cleanup, rethrow and
secondary-failure paths below are reused: their calls, arguments, values,
priority, transfers and effect order did not change. The renewed LLM judgment
finds no contradictory capture lifetime or exception identity. Structural
comparison supports this reading rather than supplying behavioral proof.

The [fresh monitor/binding challenge](../../sync/monitor-surface-boundaries/review.md)
covers capture identity, receiving/value flow, nested failures and escaping
storage. Current fixture/model regressions are separate executed evidence;
this record does not claim VM/device execution for this input.

## Prior verdict — whole-operation follow-up

Read late third-acquisition/read failures and normal completion. Acquired
handles50/51 retain their identities and close order; no unacquired third handle
is closed. The normal pure total remains computed before the same cleanup calls.
Pending read/acquisition faults and replacement cleanup faults keep the original
routing; no return is fabricated after failure.

The [pure-tail challenge](../../exceptions/pure-tail-finally/review.md) and
[carrier challenge](../../atoms/refined-carriers/review.md) record the owning
producer/consumer repairs and blocked drafts. This is bounded current source
reading; historical execution claims below were not rerun and do not certify
this output. No behavioral executor, device run or Java recompilation ran.


Three nested resources with later-acquisition, body and independent close
failures. `ThreeResourceLateFailure.java` explains the behavior;
`ThreeResourceLateFailure.smali` is the actual assembly input. The prepared
`classes.dex` was assembled from that file and copied beside it.

- Provenance: authored for this repository; no third-party code or data.
- Original owner: `Lfixture/ThreeResourceLateFailure;`.
- Subject: `Lfixture/ThreeResourceLateFailure;->openThree(I)I`.
- R1 review base: repository HEAD `6d663d9`; engine sources unchanged from
  `12de370`.
- Expected behavior: the scenario table in `README.md`, authored from the Smali
  and Java sources before this reading pass.

## Prior verdict — round 2

Manually reread the Java explanation, Smali and fresh P6/P9 delivery at
`09e322b`, using the README's native-helper models. The reviewed normal, late
acquisition, read-failure and close-failure paths agree: only acquired handles
close, in order 2/1/0; the original primary retains suppressed failures in that
order; a normal close failure becomes the next outer handler's primary.
The result is saved before closes. No new semantic mismatch was found.

The current text is unchanged from the inspected `6d663d9` engine. Its expanded
resource protocol is readable; compact resource syntax is not required for this
judgment. Fresh delivery has no stopped member or rendering issue. Behavioral
execution remains unrun.

## Prior review — round 1

### Semantics

This is a manual source/output reading judgment, not an automated execution result.

| Method | Semantics |
| --- | --- |
| `openThree` | ✔ Equivalent by path comparison: acquisition order, reverse close order, primary identity and suppression order agree. |

### Path comparison

The helper bodies are native. The review models `open(id) = id`,
`read(handle) = 10 * handle`, normal closes, and the faults listed in the README.
The current output keeps three nested `try`/`catch (all)` scopes inside the
method body. The first close in the output is handle 2, then handle 1, then
handle 0.

| Path | Reference and recovered behavior |
|---|---|
| Normal | The output evaluates O(0), O(1), O(2), R(0), R(1), R(2), then C(2), C(1), C(0) and returns the saved sum. The displayed result variable is assigned before the closes. |
| First acquisition fails | No protected region or close is entered; the output propagates the failure before the nested scopes. |
| Second acquisition fails | The outer handler closes handle 0 only. |
| Third acquisition fails | The middle handler closes handle 1, then the outer handler closes handle 0. Handle 2 is never closed. |
| Body read fails | The inner handler closes handle 2; the middle handler catches the rethrow and closes handle 1; the outer handler closes handle 0. Suppressed identities accumulate in the order C2, C1, C0. |
| Body read and every close fail | The output calls `addSuppressed` on the same captured throwable in the same inner-to-outer order; each close has its own nested suppression handler. |
| Normal close of handle 2 fails | The failure enters the middle handler, which closes handle 1 and lets the outer handler close handle 0. The remaining close failures are suppressed on the first close failure. |
| Normal close of handle 1 fails | The outer handler closes handle 0; a normal close failure of handle 0 remains outside every handler and propagates. |

An `addSuppressed` failure is not in the scenario table. Reading the output shows
that the inner suppression call is inside the middle protection and the middle
suppression call is inside the outer protection, matching the source's abrupt
exit from each catch block. That path was reviewed but not modeled as a scenario.

### Readability

The nested `try`/`catch (all)` form is readable and the closes are visibly inside
their owning scopes. Explicit `capture_exception` assignments, generated local
names and the absence of compact `try (resource...)` syntax are notation limits,
not execution mismatches. No `exception_chain` line or detached handler is needed
for this method.

### Commands

```sh
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/three-resources-late-failure/classes.dex \
  --exec '{"cmd":"p9-all","text_only":true}'
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/three-resources-late-failure/classes.dex \
  --exec '{"cmd":"p6-all","details":true}'
```

No automated behavioral execution and no Rust fixture test were run.

## Rounds

- R1: Prepared `classes.dex` from the authored Smali; direct semantic reading
  finds the resource behavior consistent with the source. The remaining limits
  are explicit notation, generated names and the unrun behavioral executor.
