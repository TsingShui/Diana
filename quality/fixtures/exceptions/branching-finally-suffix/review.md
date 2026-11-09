# Fixture: branching-finally-suffix

## Current verdict — terminal-follow conjunction, 2026-11-09

`finish` now prints the `p1 == 5 || p1 == 6` membership test before its
`IllegalArgumentException("close")`; `read` prints `p1 == 4 || p1 == 6` before
its `AssertionError`. The successful result and both throw/return paths are
unchanged; the rest of the fixture text is unchanged in the candidate comparison.

## Current verdict — terminal guard follow-up

Read the changed terminal alternatives and remaining body. The selected
return/throw keeps its original evaluation, while other copies, calls, accesses
and results remain on the opposite path in their original order. Both-edge guard
orientation preserves the predicate's exact meaning; no data, check or effect is
dropped.

Compared the same prepared DEX on 29f1b82 and this commit. P7 requires a complete
linear arm ending in a printed return/throw; opaque scopes are not exit evidence.
The [fresh guard challenge](../../branches/terminal-guard-arms/review.md) covers
actual other-edge Phi copies, parent scope, recovering catches and cleanup.
Original P2 through complete P6 products agree; this is bounded LLM path reading,
not behavioral execution. Historical execution claims below were not rerun.


## Prior verdict — while/do-while architecture correction

Read the changed catch-loop tail: original read/recovery and sum precede row++,
and normal loop completion reaches the same conditional finally. Pending
rethrow/unhandled and outside-handler result300 guards remain; no cleanup effect
or argument moves.

Compared the same prepared DEX on 8ac6d95 and this commit. Dedicated for recovery
contradicted the approved architecture and is removed, not retained behind a
switch. Updates stay in ordinary body positions with original targets/copies;
explicit gotos can be necessary instead of update-skipping while continues.
The [fresh challenge](../../loops/while-tail-continuations/review.md) covers
three-level private continuations, real shared entrances, cleanup and do-while.
This is bounded LLM source/path reading, not behavioral execution. Historical
execution claims below were not rerun and do not certify this output.


## Prior verdict — whole-operation follow-up

Read the changed protected scalar placement and conditional cleanup. Pure
preparation keeps its original position before cleanup; cleanup branches,
arguments and field/call order are unchanged. Pending rethrow is folded only
with its original unhandled delivery; the outside-handler recovery case remains
explicit and retains its result300.

The [pure-tail challenge](../../exceptions/pure-tail-finally/review.md) and
[carrier challenge](../../atoms/refined-carriers/review.md) record the owning
producer/consumer repairs and blocked drafts. This is bounded current source
reading; historical execution claims below were not rerun and do not certify
this output. No behavioral executor, device run or Java recompilation ran.


## Prior LLM verdict

Direct reading of Java, actual Smali, helper bodies, P6/P7 delivery and final
ordinary Engine text. Expectations precede candidate inspection. Baseline is
the catch/loop working tree on `ebbc003`, before the finally changes. The same
prepared DEX was used for every comparison. These are semantic reading judgments;
no device, Java recompilation or independent scenario executor ran.

| Original subject | Judgment |
|---|---|
| `Lfixture/BranchFinally;->run(III)I` | Equivalent on reviewed scenarios: loop/catch values precede one branching cleanup outside the loop, with exact helper arguments and exception replacement. |
| `Lfixture/BranchFinally;->different(I)I` | Equivalent: unequal normal/exceptional finish arguments stay distinct, not one finally. |
| `Lfixture/BranchFinally;->guardedRethrow(II)I` | Final candidate preserves the outside handler returning 300; the baseline and intermediate candidate lose this route. |
| `read(II)I`, `finish(II)V`, `mark(I)V` | Concrete bodies retain event order, mode-selected errors, row+1 return and wrapping trace arithmetic. |

### Loop and cleanup paths

The final run output initializes row/total before its try, increments the original
row after each successful or recovered value, and evaluates the same row<rows
condition. Normal mode sums 1..3 to 6. NFE/NPE share one capture, mark 20, select
100 and continue; two rows yield 200. The outer finally runs only after the loop
or an abrupt body exit, not once per iteration. flag=0 marks 40 without finish;
nonzero flag marks 41 and then finishes with the original rows/mode parameters.

An ISE/Error bypasses typed catches and becomes the pending original exception.
Successful cleanup preserves it. In mode 5 the whole normal loop completes before
finish throws IAE. In mode 6 the first read's Error is replaced by finish's IAE
when flag is nonzero, but remains Error when flag is zero. No suppression/retry is
introduced. Zero rows do not execute read: (0,6,0) returns zero after mark 40,
whereas (0,6,1) throws from finish with no prior Error to replace.

Both cleanup branch roles and the common continuation are retained. Matching
crosses only instruction-free normal routing, never an unpaired instruction.
Normal body copies remain at their original outlets before cleanup. Source
anchors retain each original call/capture even when paired cleanup prints once.

### Unequal arguments

The different method's normal finish(1,mode) marks 31, while its exceptional
finish(2,mode) marks 32. The candidate retains both: normal mode returns 1;
a read NFE is rethrown after mark 32; mode 5 fails after mark 31; mode 6 replaces
Error after mark 32. Equal operation kinds or declared view categories alone do
not equate the distinct literal values.

### Outside pending-rethrow route: discovered semantic defect

The later guardedRethrow challenge retains one outside capture shared by a prefix
fault and the inner pending rethrow. With (1,0), prefix read(99,0) marks 109 and
returns; read(0,1) marks 10 and throws NFE; mark(0) runs, then the original
rethrow dispatch reaches the outside capture and returns 300. With (0,3), only
mark 109 occurs before that capture returns 300. (0,0) returns 1 after 109,10,0.

Both the pre-finally baseline and the intermediate candidate incorrectly print
an unhandled implicit rethrow after finally. On (1,0) that representation would
propagate NFE rather than return 300: a definite mismatch by direct source/path
comparison, not an executed failure report. The final owning-stage construction
requires the hidden pending rethrow's original route to be unhandled. It now
keeps the explicit catch, mark(0), and throw with its original outside chain.
The route is not guessed from lexical proximity or erased for prettier finally.

Guarded cleanup and guarded pending-rethrow contexts remain explicit until their
natural enclosing delivery is established by an appropriate producer contract.
This conservative boundary does not suppress their available code or facts.

## Mechanism and iteration decision

The seed repair consumes the selected one-clause catch-all interval, accepts only
body-internal or exact cleanup exceptional exits, distinguishes value-preserving
IntegralView provenance from runtime conversion, and leaves incoming body edge
copies before finally. This new challenge initially failed: protection's first
iteration join was not the actual loop exit. Protection now completes an abrupt
catch-all over an eligible established loop only when every added throwing site
retains the same protection or stays internal. An unguarded cleanup call blocks
that enlargement. The cleanup cursor also normalizes a branch-specific empty
route to the same original continuation.

The candidate is effective on the reviewed seed/challenge paths after fixing the
outside-rethrow defect. That is a bounded qualitative iteration judgment, not
universal behavioral verification or authorization to commit.

## Supporting evidence

Five checks in `crates/diana-decomp/tests/finally_scopes.rs` cover selected suffix
ownership, two retained executable incoming-copy anchors before cleanup, the
branching cleanup outside the loop, the per-iteration unguarded-cleanup barrier,
unequal arguments and the explicit pending-rethrow route. They consume ordinary
Engine delivery; they do not execute these scenarios.

Final default and all-feature nextest each passed 30 package checks. Package
Clippy with warnings denied, release build, and dbg_shell build also passed.
Corpus comparison covered 78 fixtures, 129 classes and 600 methods with no stopped
members, render failures or render issues. P2 and complete delivered P3/P4/P5
products, plus original P6 control/exception/lock facts, agree. Four fixture texts
changed relative to the pre-finally working tree; NestedCatchLoops subjects are
identical, with only its copied helper scan changing. A second comparison against
fixed revision `ebbc003` covers all three repair iterations and their eight changed
fixture texts. This inspection is not behavioral execution; that remains unrun.
