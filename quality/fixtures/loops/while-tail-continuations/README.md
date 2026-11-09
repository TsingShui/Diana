# While and do-while continuation challenge

## Independent expectations

Authored after removing dedicated for/update roles and retaining exact private
loop-exit evidence, before this fixture's first inspection. The existing
LoopExits.outerContinue seed has a private tail containing an enclosing latch.
This challenge adds three nested loops, a different nonlocal row increment,
throwing tail work, a genuinely shared-entry refusal, finally before a shared
parameter update, and a first-iteration do-while.

Original owner Lfixture/WhileTails;. Subjects nested(III)I, shared(III)I,
walk(III)I, tail(II)I and concrete mark/touch/read/cleanup helpers. trace starts0;
mark appends trace*31+event with32-bit wrapping. touch marks its event and throws
ISE("tail") exactly when event==fault. read marks10+row then modes1/4 throw
NFE/Error, otherwise returns row+1. cleanup marks30+row then modes2/4 throw IAE.
Constructors/allocation succeed; asynchronous/linkage faults and concurrent
trace writes are excluded. Java explains the actual authored Smali.

| Subject / arguments | Expected exit | Ordered mark arguments |
|---|---|---|
| nested(3,0,-1) | return20 | 100,120 |
| nested(3,1,-1) | return22 | 100,120 |
| nested(3,2,-1) | return26 | 100,120 |
| nested(2,-1,-1) | return24 | 200,201 |
| nested(0,1,-1) | return0 | none |
| nested(3,1,100) | throw ISE before total+10 and row+2 | 100 |
| nested(3,1,120) | throw ISE during second nonlocal tail | 100,120 |
| shared(3,-1,-1) | return22 | 100,120 |
| shared(2,-1,100) | throw ISE | 100 |
| walk(4,1,0) | return8 | 10,30,11,31,12,32,13,33 |
| walk(3,2,0) | return4 | 10,30,12,32 |
| walk(4,1,3) | return4; break skips row update | 10,30,11,31,12,32 |
| walk(0,1,0) | return0 | none |
| walk(4,1,1) | throw original NFE after cleanup | 10,30 |
| walk(4,1,2) | throw normal-cleanup IAE | 10,30 |
| walk(4,1,4) | throw IAE replacing Error | 10,30 |
| tail(0,-1), tail(-2,-1) | return0 after first body | 60 |
| tail(3,-1) | return3 | 60,61,62 |
| tail(3,61) | throw ISE before second sum/update | 60,61 |

nested's private tail performs touch, +10 and row+2 before continuing the outer
loop. It skips inner/middle updates and normal outer touch200+row. A fault skips
all following updates. shared adds a second original conditional entrance to
that same tail; neither entrance may claim it private or duplicate it.

walk's row1 branch skips only the addition, not finally or update. Cleanup must
precede every normal update; break/throw skips update. No original update may
be hidden in a header, duplicated or skipped by replacing its target with a
while continue. tail must execute its body once even for a nonpositive limit.

## Preparation

Assemble through the repository Smali procedure using API24 in the ignored
fixture cache and copy only classes.dex beside these sources. Compare the same
prepared DEX on baseline8ac6d95 and candidate via ordinary dbg_shell. Prepared
using Smali2.5.2, API24 and OpenJDK21.0.12.1. Four ordinary-Engine product checks
pass; [manual review](review.md) records the bounded source-path judgement. No scenario executor, device run
or Java recompilation is claimed.
