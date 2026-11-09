# Mutable iteration operands through cleanup

## Independent expectations

Authored after the nonconstant-read P6 repair, before this fixture's first
baseline/candidate inspection. Discovery: stride(II)I in
[iteration-tail-cleanup](../iteration-tail-cleanup/README.md). The retained trigger
is a shared pure self-update reading OTHER original local storage. Added
interaction: that local changes in the body (2 after row0, then1), shared typed
recovery, outer cleanup and update-reaching versus update-skipping exits.

Owner Lfixture/MutableStep;. Subjects walk(III)I, divide(II)I and concrete helpers
mark(I)V, read(II)I, cleanup(II)V. All helpers exist in the prepared DEX. trace
starts at0 and wraps as int. Standard exception construction succeeds; no
asynchronous/linkage/allocation fault or concurrent trace mutation is modeled.

read marks10+row then returns row+1 normally. Modes1/2 throw NFE/NPE, recovered
by one original shared handler after mark20, selecting100. Mode3 throws ISE;
modes4/6 throw Error. cleanup marks30+row, then modes5/6 throw IAE. Other modes
succeed. A cleanup failure must not run the update, next read or normal suffix.

| Subject | Arguments | Expected exit | Ordered events |
|---|---|---|---|
| walk | (5,0,0) | return4 | 10,30,12,32,13,33 |
| walk | (5,0,1) | return3 | 10,30,12,32,13,33 |
| walk | (5,0,2) | return0 | 10,30,12,32 |
| walk | (5,1,0) or (5,2,0) | return200 | 10,20,30,12,20,32,13,20,33 |
| walk | (5,1,1) | return100 | 10,20,30,12,20,32,13,20,33 |
| walk | (5,2,2) | return0 | 10,20,30,12,20,32 |
| walk | (5,3,0) | throw original ISE | 10,30 |
| walk | (5,4,1) | throw original Error | 10,30 |
| walk | (5,5,2) | throw normal-cleanup IAE | 10,30 |
| walk | (5,6,0) | throw IAE replacing Error | 10,30 |
| walk | (0,6,1) | return0 | none |
| divide | (0,2) | return0 | 4,2,99,1,99 |
| divide | (0,0) | throw ArithmeticException | 4 |
| divide | (4,0) | return4 | none |

No loop-invariance or progress fact may be invented for an update operand.
walk must read the current step at its original point after cleanup and before
retesting. row0's continue reaches step2; row3's break and flag2's row2 return
skip the update. Original read/capture/effect ordering and values remain.
divide's shared throwing update also remains an ordinary body operation;
there is no pure-update classifier or header-placement permission.

## Nested boundary challenge

Added nested(III)I AFTER the initial candidate's lost walk boundary was traced
and repaired, and BEFORE nested's first baseline/candidate inspection. It retains
the other-local inner update and adds two loops, a nonlocal break, a nonlocal
continue, an inner continue and observable work after the inner loop. The first
candidate had collapsed no-body boundary exits into no-body common follows;
a compact condition must now preserve those different original completions.

Independent scenarios (rows,step,flag): (2,1,0) returns4 with events
0,1,2,90,10,11,12,91; (2,1,1) returns1 with0,1 only; (2,1,2) returns0 with0,10
only; (2,2,1) returns2 with0,2,90,10,12,91; (2,2,2) returns0 with0,10;
(0,0,2) returns0/no events. A nonlocal continue skips the inner update and
outer-tail mark, but reaches the original outer update. A nonlocal break skips
both updates and tail marks. No zero-step progress claim is made.

## Preparation

Assemble with the repository Smali procedure, API24, into
the ignored fixture cache and copy only classes.dex here. Compare this same DEX
with the recorded pre-repair binary and candidate through ordinary dbg_shell.
Prepared with Smali2.5.2, API24 and OpenJDK21.0.12.1. Baseline is the recorded
working tree based on ebbc003 after protected-prefix completion and before
nonconstant update admission. Both comparisons use this same prepared DEX.
Current comparison uses 8ac6d95 and the commit containing this update on the
same DEX. All loops use while/do-while and visible body updates. Nested break
retains its outer scope; the update-reaching exit keeps a goto to the outer
update instead of a Continue that skips it. divide stays an ordinary while. See review.md
for the initial blocked regression, final LLM reading and check evidence.
No behavioral executor or Java recompilation is claimed.
