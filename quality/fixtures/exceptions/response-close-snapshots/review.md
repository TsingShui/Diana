# Response and resource cleanup review

## Current verdict — scoped catch capture

Compared the same prepared input on `c95c6dd` and this repair. Reviewed header
bindings: `all v3`, `all v4`. The original captured object is bound once at exceptional
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

## Prior verdict

The reduction reproduces R2's explicit response `ProtectionBoundary` graph and
before-throw response snapshots on baseline df10b744985e3407f902442b8252fab100700aec.
The resource method retains explicit body/close/suppression catches. The scalar
repair leaves both texts and all P2–P8 products unchanged; a natural resource or
finally scope is not claimed.

### Semantics

These are bounded LLM judgments under the README's native/library model, not
executed behavioral acceptance.

| Method | Judgment and reasoning |
|---|---|
| `response(I)I` | ~ Before acquire, all selected handler snapshots contain null. After acquire commits, ready/read snapshots retain that acquired handle. Interruption has priority over IOException. Normal success captures77 before close; false ready skips read and returns-1 after close. IOException logs before its close/null return; interruption and other failures close before rethrow. |
| `resource(I)I` | ~ Acquisition failure precedes body protection, so it cannot close an absent handle. Read success captures the result before normal close. Read failure captures the primary, conditionally closes, catches a secondary and passes it to primary.addSuppressed before rethrowing that same primary. |
| `acquire(I)ViewHandle` | ? Native model records acquisition mode and returns identity1000+mode, null or the selected failure. It is not executed or parsed into the Engine. |
| `log(Throwable)V` | ? Native model receives the original IOException object. A logging failure enters cleanup with the same acquired handle, then propagates unless cleanup replaces it. |
| `ViewHandle.ready()Z` | ? Modeled canonical boolean/failure after its trace event. |
| `ViewHandle.read()I` | ? Modeled return77 or specified failure after its trace event. |
| `ViewHandle.close()V` | ? Modeled single close of the actual handle, with the scenario's selected failure identity. |

In response, acquire failures select null snapshots and cannot close identity7
or a fabricated handle. Later failures save identity1007. Cleanup on normal,
IO-recovery, interruption and other exceptional paths has one selected close;
its own call lies outside the cleanup handler, so a throwing close replaces the
pending result/throwable rather than looping or adding a suppressed exception.
A failing log also saves the acquired response before entering catch-all cleanup.

In resource, a successful-body close failure replaces return77. A body failure
A plus distinct close failure B instead records B on A and rethrows A. If B is
the same object as A, addSuppressed itself rejects self-suppression; that new
failure propagates before the later rethrow. The renderer does not merge the
primary and secondary storage reads or invent a second close.

Two ordinary-Engine checks confirm ordered handler types, null/handle snapshot
sources and distinct primary/secondary argument occurrences. Origin checks retain
each original call/capture. The full test set passes but does not execute these
native models or prove arbitrary cleanup equivalence.

## Rounds

- Reduced ordinary-finally and resource-suppression paths with retained snapshot
  phases; natural cleanup recovery remains separate from C1's scalar repair.
