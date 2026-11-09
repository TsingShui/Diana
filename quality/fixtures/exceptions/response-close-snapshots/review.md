# Response and resource cleanup review

## Combined verdict

Integration 271eb47 retains concurrent recovery through cfc9da3. Compared with
that latest-main baseline on the same prepared bytes, response gains its two
natural scopes; resource text is unchanged. Direct reading of the integrated
headers binds v4/v5/v6 once before their original receives and body work. Null/
handle snapshots, typed priority and outward normal-close targets remain exact.
Relative to the isolated P6 repair, resource's private primary/secondary captures
also bind in their headers, with distinct values and unchanged suppression/
rethrow order. Both combined configurations pass 164 checks; original P2–P5 and
P6 coordinates agree. No R1/R2 native model or Java/DEX execution is claimed.

## P6 ownership observation against 73fc879

Against fixed baseline 73fc879, response now exposes two natural catch scopes
without acquiring its unguarded normal/IO close suffixes. Read the same source,
Smali, independent scenario table and complete before/after output. Acquire
failure still receives null; later sites receive the committed handle. Typed
interruption remains before IOException. IO logging can replace the original
failure before canonical cleanup; a throwing close replaces the pending result/
throwable without retry or suppression. Null-skipping branches keep their actual
later return targets rather than falling into another close. The resource method
is byte-identical in text and retains its original primary/suppressed behavior.

P2–P5 and original P6 nodes, links, clauses, completions, copies and locks remain
exact. The original preservation checks and new natural-scope/outlet check pass.
Fresh [dual outlets](../cleanup-outlet-frontiers/review.md) and
[shared contexts](../shared-cleanup-contexts/review.md) cover broader interactions.
Both isolated configurations pass 155 checks. These are LLM reasoning and product
checks; no native model, DEX VM or decompiled Java is executed.

## Scoped capture observation against c95c6dd

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

## Scalar-use baseline observation

The reduction reproduces R2's explicit response `ProtectionBoundary` graph and
before-throw response snapshots on baseline df10b744985e3407f902442b8252fab100700aec.
The resource method retains explicit body/close/suppression catches. The scalar
repair left both texts and all P2–P8 products unchanged; natural protection is
provided by the separate P6 repair above.

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
  phases; the separate P6 ownership repair now recovers response's natural scopes.
