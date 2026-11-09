# Fragment source/output review

LLM semantic review compares the authored Java/Smali and ordinary dbg_shell output
on the retained `d5c665e` binary and the current refactor. The output text and P6
products agree. The source-origin assertions are independent expectations;
executed checks cover ownership, recovery structure and origin retention, not VM
or final-plan behavior.

In branchLoop, the original zero assignment precedes the flag test. Flag zero
returns 7 and skips the loop; other flags enter one while with a bound of 3.
The update remains inside that loop and its next operation is the original
predicate. Starting at zero, three updates produce 3. There are no calls or
exception phases. The omitted constants 3 and 7 now retain zero-width original
operation/statement anchors; their rematerialized uses do not execute stores.

In irreducible, the initial flag chooses the existing A or B label. The A update
adds 1 and either branches to B or goes to the existing return. B adds 2 and
either branches to A or falls through to that same return. Thus flag zero follows
0 -> 1 -> 3 -> 4, while nonzero follows 0 -> 2 -> 3 -> 5. Both original update
occurrences remain once in the graph; no dispatcher or copied half-cycle is
introduced. The common ordering kernel retains the cycle rather than presenting
it as a Sequence. Constant 4 now has its missing source anchor.

In selfCycle, the nop/backward goto has no effect or exit. The delivered
while-true loop preserves that behavior, and inspection returns without executing
the loop. The original nop keeps its zero-width source anchor. No zero-delta
branch remains in the published input.

No contradiction was found on these paths. Full historical regression and VM
execution remain unrun. The separate guarded-switch challenge adds shared keys
and repeated loop transfers without changing the contraction mechanism.
