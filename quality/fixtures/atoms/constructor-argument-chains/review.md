# Fixture: constructor-argument-chains

## Current verdict — scoped catch capture

Compared the same prepared input on `c95c6dd` and this repair. Reviewed header
bindings: `all v7`. The original captured object is bound once at exceptional
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

## Prior verdict — ordered constructor argument recovery

Read the complete Java/Smali input, helper model, baseline `e95b6a4` and candidate
P9 texts. P1/P3/P4 publications remain identical. P5 now publishes the complete
six-producer argument trees and equal protection; P7 lowers those products and
P9 renders them. All original members and the three native declarations remain.

### Semantics

| Subject | Path reasoning | Judgment |
|---|---|---|
| `point()Lfixture/Point;` | Allocation remains before G0,D0,width120, then G1,D1,height480, then initialization. The two native getSystem calls are independent; each receiver and field access uses its own returned object. Result P has x120,y480 under the model, not width/height from one coalesced metrics object. Any selected G/D failure or null receiver/metrics prevents all later steps; constructor success is required before publication. | Agreement for every recorded argument-success/failure path; the split allocation/initialization is recovered as one New. |
| `caughtPoint()Lfixture/Point;` | The same ordered expression executes in the original try. Allocation, six preparations and initialization retain one catch-all chain and all eight original sites. A failure reaches note with the exact caught error and returns null. Point is returned only on normal construction completion; its declaration remains outside try because that successful read is outside the try scope. A note failure escapes. | Agreement for normal, argument, allocation/constructor and note outcomes under the model. |
| `bodyError(I)V` | Mode0 returns before allocation. Mode1 allocates, loads `body`, initializes and throws ISE. The same explicit c0 chain handles every original throwing position and the terminal. On construction/string failure it receives that failure, rather than an invented completed ISE. The fresh throw composes into `throw new ... [exn c0]` while H6/note remain explicit. | Agreement; compact construction does not claim newly recovered catch structure. |
| `Point.<init>(II)V` | Calls Object's constructor before x and y field stores, in that order. Its complete output is unchanged. | Agreement under the normal Object-constructor assumption. |
| Native helpers | G0/G1 and D0/D1 behavior and failure identities use the explicit README model. note remains outside the protected construction corridor. No external declarations are loaded by the engine. | Reviewed model boundary, not executed native behavior. |

P5's original single-preparation rule rejected the first call's intermediate use
and required a reference initialization flag for numeric values. The new rule
consumes exact producer/use edges, treats reference initialization separately
from numeric views, and accepts only the complete original operand postorder.
Every throwing member keeps equal ordered handler delivery; snapshots are
collected across all members. P7 preserves read origins without adding a spurious
allocation-receiver read. Prepared expressions own their operation/result anchors;
the group owns the complete site set, avoiding duplicate anchors.

The six ordinary-Engine test groups preserve original calls, result commits,
folded-member and throwing-operation anchors, catch delivery and refusal cases.
The test's scoped initializer assertion accounts for fully qualified types in
method retrieval. Standalone folded literals outside the recovered group are
not claimed as part of this origin check. The fresh
[effect challenge](../constructor-argument-effects/review.md) adds branching
expression trees, interleavings and original failure-state saves.

### Evidence and decision

This is primary LLM path-by-path review, supported by product/origin tests and
same-input output comparison. The independent evaluator in the tests executes
numeric argument trees on the challenge only. It does not execute this fixture's
native resources/metrics, method control, catches or external constructors.
No full-method VM/device run or Java recompilation is claimed. Allocation and
external constructor fault outcomes are reviewed against original delivery and
the recorded assumptions, not described as executed fault injection.

No contradiction was found in the required paths. The matching challenge and
prior changed fixture reviews complete the bounded semantic-review portion;
implemented dev regression checks remain separate acceptance evidence.
