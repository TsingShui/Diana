# Fixture: abrupt-finally-completion

## Current verdict — scoped catch capture

Compared the same prepared input on `c95c6dd` and this repair. Reviewed header
bindings: `all v2`, `all v4`, `all v4`. The original captured object is bound once at exceptional
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

## Prior verdict — abrupt cleanup correspondence

The same prepared DEX was inspected on baseline `b214ec6` and this repair.
Read `CleanupExits.java`, every Smali subject and helper, and both complete P9
class texts. The baseline duplicates cleanup through catch-all paths; the
candidate selects one finally for `capture`, `always`, `voidAlways`, `bareVoid`,
`parameterThrow` and `twoRethrows`. The three refusal subjects remain explicit.
This is primary LLM path-by-path review. The six ordinary-Engine tests supply
separate product, saved-state, refusal and origin evidence; they do not execute
complete method behavior.

### Semantics

| Original subject | Reviewed paths and reasoning | Judgment |
|---|---|---|
| `mark(I)V` | Reads trace, multiplies by 31 and adds the event in wrapping 32-bit arithmetic before its one field store. This helper's complete output is unchanged. | Agreement under the recorded initial trace. |
| `read(II)I` | Marks `10 + stage` before testing `fault & (1 << stage)`. Success returns `stage + 7`; failure constructs and throws a fresh ISE with message `read`. Output and its calls are unchanged. | Agreement for both branches. |
| `close(II)V` | Marks `30 + stage` before testing `fault & (16 << stage)`. Success returns void; failure constructs and throws a fresh IAE with message `close`. Output is unchanged. | Agreement for both branches. |
| `capture(III)I` | The body read precedes the saved `read + base` add. Its first AddInt has different operands from cleanup's AddInt and stays in the protected body. Cleanup computes `base + 100`, closes stage0, then marks the changed base. Normal completion returns the saved body state: `(0,1,0)` returns8, not108. Bit2 returns -7 before bit4 can construct/throw the override IAE. A read error runs identical cleanup; ordinary cleanup rethrows the same error, while an override replaces it. A close error occurs before the changed-base mark and all later overrides, replacing either pending result or error. | Agreement on every README scenario; causal trigger retained and improved. |
| `always(II)I` | Both normal read and read failure enter close0. Close failure exits immediately. Otherwise bit4 throws the fresh override IAE; its other branch returns8. All successful cleanup paths replace the pending exit, so there is no implicit rethrow or invented follow. | Agreement for normal, read-fault and close-fault paths. |
| `voidAlways(I)V` | Normal read and read failure both close0 and then return void. This bare return discards the pending read error. A close error prevents the return and remains the selected error. | Agreement for all four recorded fault states. |
| `bareVoid(I)V` | Read executes once and marks10. Both its success and failure finish through a single finally return, with no close. The read failure is swallowed by that return. The original terminal-only cleanup is retained, rather than requiring a fabricated operation pair. | Agreement for both read outcomes. |
| `parameterThrow(Ljava/lang/Throwable;I)V` | Read still executes before cleanup. Cleanup throws the exact supplied parameter on either body outcome; it does not throw the hidden capture or allocate a replacement. A null parameter retains the original throw-null NPE behavior. | Agreement for non-null identity and null scenarios. |
| `twoRethrows(II)I` | Mode bit1 chooses close0, otherwise close1. Both original capture throws become implicit only on their own normally completing cleanup path; the actual normal return8 is their common continuation. Close0/close1 failures replace the pending read error, with marks30/31 respectively. The output's if/else executes one selected close, never both. | Agreement for both selections, original error identity and reachable close failures. |
| `differentReturn(I)I` | A normal read closes then returns -7. A read error is caught, closes, and returns -8. Close errors replace either path's result. These unequal terminal reads are not folded; both constants remain visible. | Correct refusal. |
| `differentThrow(I)V` | Normal read closes then constructs IAE(`normal`); caught read failure closes then constructs IAE(`handler`). The message arguments differ. Both paths and their distinct constructions remain visible. | Correct refusal. |
| `foreign(II)I` | Bit8 bypasses read and reaches the original normal close0/return8. Bit8 clear runs read; a read error takes the separate catch close0/return8. Close faults propagate from either entry. The shared normal cleanup has an outside predecessor and stays explicit. | Correct refusal; bypass and effects retained. |

### Recovery and producer obligations

The new fixture adds captured return preparation, parameter mutation, throwing
close, constructed exceptions, always-abrupt/void exits and multiple pending
rethrows to the discovery's paired cleanup trigger. Operand/initialization
correspondence establishes the first cleanup frontier. P6 consumes complete
selected P5 construction groups and pairs return/throw terminals; it does not
construct P5 facts. P7 makes only duplicate exits and pending rethrows origin-only.
Canonical cleanup exits remain executable and each original terminal keeps one
anchor. P9 consumes the delivered layout through the existing renderer.

P1/P3/P4/P5 publications are identical between revisions for this same DEX.
All subjects and helpers remain readable. The three refusal cases are reviewed
compatibility guards, not claimed recovery improvements.

### Evidence limits and iteration decision

The review assumes fresh zero trace, sequential execution and the concrete helper
bodies. No asynchronous exception, allocation failure or external constructor
failure is assumed in the scenario results. The input's allocation/constructor
order and uncaught routes remain, but those failures were not behaviorally
executed. No VM, device run, Java recompilation or whole-method behavioral
executor was run. The written judgment does not constitute behavioral PASS.

Within those scenarios, the repair generalizes to the new complexity challenge
without a contradictory path. The original discovery and all other changed
old-fixture paths receive renewed reviews beside their inputs. Unchanged input
and exact output reuse earlier independent reviews; fresh dev regression checks
remain a separate integration obligation.
