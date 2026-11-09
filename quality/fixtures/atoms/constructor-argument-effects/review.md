# Fixture: constructor-argument-effects

## Current verdict — scoped catch capture

Compared the same prepared input on `c95c6dd` and this repair. Reviewed header
bindings: `all v3`, `all v5`, `all v9`. The original captured object is bound once at exceptional
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

## Prior verdict — branching trees and failure-state guards

Read every subject in Java/Smali, native helper models and both complete P9 texts
for the same DEX on `e95b6a4` and the candidate. P1/P3/P4 agree. The two positive
subjects now use complete natural construction; all five intentional guards keep
their original explicit allocation, effects and initialization.

### Semantics

| Subject | Path-by-path reasoning | Judgment |
|---|---|---|
| `tree(II)Lfixture/Point;` | Allocation precedes R0,R1,add15,F(15),R2,add(9+base), then initialization. With base5 and no fault, fields are30 and14. A read0 failure skips R1/F/R2; read1 skips F/R2; finish failure skips R2; read2 skips initialization. Every fault reaches note(base,error) through the original catch and returns null. Constructor failure does not publish Point; note failure escapes. | Agreement. The complete producer tree retains the discovery trigger under multiple operands, arithmetic and a later parameter read. |
| `caughtError(I)V` | Allocation precedes R0, then M(7), then IAE initialization and throw on success. Read or message failure skips later preparation/initialization; the catch notes the actual failure. Success notes the fresh IAE with message `value=7`. note's own failure escapes. The printed explicit c0 marker represents the same original ordered chain for all composed sites and the throw. | Agreement for the modeled read/message/throw/catch paths. |
| `reversed(I)Lfixture/Point;` | Allocation occurs before R1 then R0; constructor consumes R0 as x and R1 as y. Normal result is7,8. Fault2 stops after R1; fault1 follows R1,R0. Natural left-to-right constructor arguments would change that order, so the original explicit sequence stays. | Correct refusal. |
| `shared(I)Lfixture/Point;` | Allocation precedes one R0; both fields use that one value7. Two argument evaluations would duplicate the call and change fault/effect behavior. The shared value and explicit constructor remain. | Correct refusal. |
| `interleaved(I)Lfixture/Point;` | Allocation precedes R0,R1,F(7), then initialization with14,8. In particular, read1 failure skips finish. Folding the first argument's read0/finish tree before read1 would contradict that path. The original order stays explicit. | Correct refusal. |
| `differentProtection(I)Lfixture/Point;` | Allocation, read0 and initialization enter A/note10 on failure; only read1 enters B/note20. Normal R0,R1 yields7,8. The complete output preserves both destinations and the exact failing error. A note error escapes. | Correct refusal; equal constructor endpoints do not erase a different argument handler. |
| `changingSnapshot(I)I` | The allocation site saves0. After allocation, saved becomes1; read/initialization sites save1. Normal completion returns1; read/init failure notes1 and returns1; allocation failure notes0 and returns0. The output retains the separate before-effect saves and handler entry copy, so it does not collapse them into a common prelude. | Correct refusal; actual save/copy steps remain. |
| `Point.<init>(II)V` | Object initialization precedes x then y writes. Complete output is unchanged. | Agreement under the recorded Object-constructor assumption. |

The independent test evaluator follows final composed New argument expressions,
models read/finish from their computed inputs and performs wrapping AddInt. It
actually checked the no-fault argument vector [30,14] and all three read-fault
prefixes. Unknown operations reject checking; original expected values and event
traces come from the README model. This is argument-expression execution, not
method/catch/constructor execution. It does not supply the note or allocation
outcomes in the table as executed observations.

All five guard method texts are unchanged from baseline. Tests separately inspect
original folded/throwing operations, result commits and sites, preserving one
anchor each even when a construction is composed into throw. Old protected-loop
regression now locates its original executable throw span rather than an obsolete
temporary-variable spelling; its loop, probe/read, update and cleanup constraints
are retained.

### Evidence limits and iteration decision

Native helpers and external constructor failures use explicit models. The Java
allocatePoint/initializePoint notation in guard explanations stands for the two
DEX operations, not added assembly calls. No VM/device run or full-method executor
was run. LLM review found no contradiction on the required paths; order/sharing
and handler/snapshot refusal boundaries remain visible. The new complexity
challenge demonstrates generalization beyond the original two linear chains.
Written review, executed argument projection and dev regression outcomes are
separate evidence, not whole-program or behavioral-regression certification.
