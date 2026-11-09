# Pure normal preparation before finally

## Independent expectations

Authored AFTER pure-tail completion and first-condition-run boundary repairs,
BEFORE this fixture's first inspection. Discovery: walk(III)I in
[mutable-iteration-step](../../loops/mutable-iteration-step/README.md). Retained
trigger: a shared typed read/recovery joins a nonthrowing normal tail before its
normal cleanup copy. Added interactions: two scalar diamonds, a parameter-Phi
assignment before cleanup, and an OR condition after cleanup in the SAME original
block. No matched cleanup operation may be skipped to align the graphs.

Owner Lfixture/TailPreparation;. Subjects finish(IIIII)I, foreign(III)I and concrete
mark(I)V/read(II)I/cleanup(II)V helpers. Helpers match the discovery: read marks10+
code, returns code+1 normally, modes1/2 throw NFE/NPE, mode3 ISE, modes4/6 Error.
cleanup marks30+code, then modes5/6 throw IAE. trace starts0 and int arithmetic
wraps32 bits. Constructors succeed; no asynchronous/linkage/allocation fault or
concurrent trace write is modeled.

| Subject arguments | Expected exit | Ordered mark arguments |
|---|---|---|
| finish(3,0,0,0,0) | return7 | 13,33 |
| finish(3,0,1,1,0) | return3 | 13,33,50 |
| finish(3,0,-1,0,1) | return0 | 13,33,50 |
| finish(3,1,0,1,0) or finish(3,2,0,1,0) | return103 | 13,20,33,50 |
| finish(3,1,1,0,0) | return99 | 13,20,33 |
| finish(3,2,-1,0,1) | return96 | 13,20,33,50 |
| finish(3,3,0,1,1) | throw original ISE | 13,33 |
| finish(3,4,-1,1,1) | throw original Error | 13,33 |
| finish(3,5,1,1,1) | throw normal-cleanup IAE | 13,33 |
| finish(3,6,0,1,1) | throw IAE replacing Error | 13,33 |
| foreign(3,0,0) | return5 | 13,33,50 |
| foreign(3,0,1) | return6 | 14,33,50 |
| foreign(3,1,0) | throw original NFE | 13,33 |
| foreign(3,1,1) | throw original NFE | 14 |
| foreign(3,5,1) | throw normal-cleanup IAE | 14,33 |
| foreign(3,6,0) | throw IAE replacing Error | 13,33 |
| foreign(3,6,1) | throw original Error | 14 |

finish's value/delta work and any actual Phi copies must execute before cleanup.
A cleanup failure skips the later OR/mark/return; no retry or suppression is added.
On success the OR remains compact without reacquiring the already owned normal
cleanup prefix from that original block. The negative flag's parameter-to-Phi
assignment must be established from actual P5 steps before claiming copy coverage.

foreign has a real outside normal arrival at the shared value+1 tail, from an
unprotected throwing read. That tail cannot become private guarded content or
cause the foreign read's exception to acquire cleanup. Its local catch-all and
normal code must stay readable even when no finally is established.

## Effect-portion and scoped-completion challenge

Initial finish established a real parameter-to-Phi Assign, but its final sum
shared a P6 portion with cleanup, so whole-node ownership still prevented finally.
It was retained unchanged while the owning portion producer gained one
whole original operation per portion. That recovery exposed CaseEntryJoins.route's hidden-normal-
cleanup goto: a bypass was not a lexical Try completion. The producer now plans
that existing body exit against pending siblings; P7/P9 do not retarget it.

Added bypass(IIIII)I AFTER portion, source-run, scoped-completion and adjacent-
expression consumer repairs, BEFORE its first inspection. It adds a switch bypass
of the shared typed read, pure value+2 before cleanup, and a normal-only value+3
AFTER cleanup in the same original block as its OR test. A bypass must complete
the try body and execute finally once, not jump to its hidden duplicate label.
The post-cleanup addition must stay outside the try; an exceptional cleanup path
must not execute it.

Independent arguments/exit/events: bypass(3,0,0,0,0) returns2 with9,33;
(3,0,1,1,0) returns9 with1,13,33,50; (3,2,2,0,1) returns105 with2,13,20,33,50;
(3,3,1,1,1) throws original ISE with1,13,33;
(3,5,0,1,1) throws normal-cleanup IAE with9,33;
(3,6,1,1,1) throws IAE replacing Error with1,13,33;
(3,6,0,1,1) throws NORMAL-cleanup IAE with9,33 (no read Error is reached).
No new behavior executor is claimed.

## Literal cleanup-prefix challenge

The first blind pure-tail prototype absorbed a literal argument belonging to
cleanup, changing switch ownership and missing finally. It was replaced: one
paired cleanup cursor supplies the FIRST event; pure body completion stops at its
first complete single-frontier occurrence, and a later correspondence failure
does not search another start. Only a full match commits preparation as body.

Added literal(IIIII)I AFTER that repair and BEFORE inspection. It retains finish's
two diamonds/parameter-Phi copy, adds an XOR result, and has TWO cleanup calls:
cleanup(7,mode) then cleanup(code,mode). The first literal7 belongs to cleanup,
not the pure body. Both calls and their argument preparation must match exactly.

Extended concrete helpers for this new subject before inspection: read also
throws Error in mode8; cleanup modes7/8 throw IAE only when code==3. Modes0..6
retain all previous expectations. This makes a fault at the SECOND cleanup
reachable, rather than incorrectly labeling a first-cleanup failure as late.

Independent literal scenarios: (3,0,0,0,0) returns6 with13,37,33;
(3,0,1,1,0) returns2 with13,37,33,50;
(3,0,-1,0,1) returns1 with13,37,33,50;
(3,1,0,1,0) returns102 with13,20,37,33,50;
(3,2,-1,0,1) returns97 with13,20,37,33,50;
(3,3,0,1,1) throws original ISE with13,37,33;
(3,5,0,1,1) throws FIRST normal-cleanup IAE with13,37;
(3,6,0,1,1) throws FIRST cleanup IAE replacing Error with13,37;
(3,7,0,1,1) throws SECOND normal-cleanup IAE with13,37,33;
(3,8,0,1,1) throws SECOND cleanup IAE replacing Error with13,37,33;
(4,7,0,1,0) returns9 with14,37,34,50;
(4,8,0,1,0) throws original Error with14,37,34.
No cleanup is retried; mark50 and return occur only after both successful calls.

## Preparation

Assemble once through the repository Smali procedure, API24, into the ignored
fixture cache; copy only classes.dex here. Compare that same DEX using the saved
pre-repair binary (completed11, now commit bda9a49) and candidate via ordinary
dbg_shell. Prepared with Smali2.5.2, API24, OpenJDK21.0.12.1; generated cache
output is ignored and only the prepared classes.dex is committed here. The
same-DEX comparison and [manual review](review.md) are complete. Package checks
inspect products/anchors, not scenario execution. No behavioral executor or Java
recompilation is claimed.
