# Terminal guard arms without an else wrapper

## Independent expectations

Authored after the P7 guard-placement repair and before this fixture's first
inspection. Discovery: LoopCatchFinally.scan(II)I in typed-catch-loop-exits.
A selected arm that actually returns/throws cannot normally enter its other arm.
The false arm is still required: its copies, body, names and enclosing cleanup
must remain, even when its else wrapper disappears.

Original owner Lfixture/GuardArms;. Subjects select(III)I, outer(III)I,
recover(II)I, loop(III)I, voided(I)V and concrete helpers. mark appends an event
as trace*31+event, with32-bit wrapping, starting at0. probe(code,mode) marks10+code,
throws ISE("test") in mode9, otherwise returns code==0. read marks20+code,
throws ISE in mode1, Error in mode3, otherwise returns code+7. cleanup marks40+code
then throws IAE in modes2/3. Constructors succeed; asynchronous/linkage/allocation
faults and concurrent trace mutation are excluded.

select saves the original base, then either selects9, returns read(1), or reaches
the shared mark50/selected+savedBase tail with the original base. Any actual
false-edge parameter-to-Phi Assign must be confirmed in P5 before claiming copy
coverage. Its requirement/Transfer/Copy origins must remain exactly once after
the guard, not at the predicate or in both places.

| Subject arguments | Expected exit | Ordered mark arguments |
|---|---|---|
| select(0,5,0) or (0,5,9) | return14 | 50 |
| select(1,5,0) | return8 | 10,21 |
| select(2,5,0) or (2,5,1) | return10 | 11,50 |
| select(2,2147483647,0) | return-2 with wrapping | 11,50 |
| select(1,5,1) | throw original ISE from read | 10,21 |
| select(1,5,3) | throw original Error | 10,21 |
| select(2,5,9) | throw original predicate ISE | 11 |
| outer(0,1,0) or (0,1,1) | return7 | 10,30,70 |
| outer(1,0,0) | return10 | 11,10,23 |
| outer(1,1,0) | return8 | 11,11,50,70 |
| outer(1,0,1) | throw original ISE from read | 11,10,23 |
| recover(0,0) | return9 | 10,22 |
| recover(0,1) | return4 after internal catch | 10,22,60,70 |
| recover(1,1) | return4 through else | 11,50,70 |
| recover(0,3) | throw original Error | 10,22 |
| loop(2,7,0) | return15 | 3,20,4,21,42 |
| loop(2,1,0) | throw guard ISE after first update | 9,20,10,42 |
| loop(2,0,0) | throw guard ISE before first read/update | 10,42 |
| loop(2,7,1) | throw original read ISE | 3,20,42 |
| loop(2,7,2) | throw normal-cleanup IAE | 3,20,4,21,42 |
| loop(2,7,3) | throw IAE replacing read Error | 3,20,42 |
| loop(2,7,9) | throw original predicate ISE | 3,42 |
| loop(0,0,0) | return0 after cleanup | 40 |
| voided(0) | return before false work | 10,30 |
| voided(1) | return after false work | 11,50,70 |

outer's false arm contains an inner guard AND later work. It is not a single
printable If suitable for else-if: when the outer true branch runs, mark50 must
not execute. recover's true arm can finish normally after its internal catch,
so it must retain else. A return keyword inside that Try does not prove escape.
loop keeps every body update and finally outside the guard's false-only body
as appropriate; its first failing predicate/throw/read must still clean once.
voided's early void return must not become an omitted method-tail return.

## Post-repair cleanup and throw challenge

The first inspection found select/outer's terminal arm on the opposite original
DEX edge; guard placement now preserves either edge's polarity and copies. The
recovering-Try guard is deliberately conservative across the WHOLE arm, not only
its last source block. The earlier inspection is not claimed as complete coverage.

Added after(II)I and throwing(II)I after those general repairs, before inspecting
these new subjects. after reads under one catch-all, cleans once, then probes
value-8 with mode0 and returns value on a match or marks50/returns value+1.
throwing's original taken edge constructs/throws, while its other path marks50
then reads/returns. No previous scenario values or helper behavior changes.

Independent after scenarios: (1,0) returns8 with21,41,10; (2,0) returns10 with
22,42,11,50; (2,1) throws original read ISE with22,42; (2,2) throws normal-cleanup
IAE with22,42; (2,3) throws IAE replacing Error with22,42. No probe/mark50 follows
a failed cleanup. Independent throwing scenarios: (0,0) throws guard ISE with10;
(1,0) returns8 with11,50,21; (1,1) throws read ISE with11,50,21; (1,3) throws Error
with11,50,21; (1,9) throws predicate ISE with11. No effect may move across the
predicate, construction or selected return/throw.

## Flattened declaration scope

Added declared(II)I after the complete-arm and polarity repair, before its first
inspection. probe(flag,mode) selects an early return0; the remaining straight
line reads2, marks that returned value, and returns value+1. Its read-result
declaration belongs to the enclosing scope after the guard, not a removed else
scope, while assignment/call timing remains unchanged. Independent scenarios:
declared(0,0) returns0 with10; declared(1,0) returns10 with11,22,9;
declared(1,1) throws read ISE with11,22; declared(1,3) throws Error with11,22.
The second mark must not execute after a failed read.

## Protected-loop follow-up

The later [protected-loop challenge](../../exceptions/protected-loop-exits/README.md)
repairs the missing P6 scope producer for loop. Its existing behavioral matrix is
unchanged. The ordinary-Engine check now requires that enclosing P6 finally and
private throw corridor before observing the Guard; this is not a parallel P7
recovery route. Fresh foreign-entry and recovering-handler cases retain the
corresponding refusals. Baseline7354242 versus this candidate uses the same DEX.

## Preparation

Use the repository Smali procedure with API24 in the ignored fixture cache,
copy only classes.dex here, and compare that same DEX on baseline29f1b82 and
candidate via ordinary dbg_shell. Prepared with Smali2.5.2, API24 and
OpenJDK21.0.12.1; generated cache output stays ignored. Nine product checks and
[manual review](review.md) cover the actual assignments, scopes and paths. No behavioral executor, device run or Java recompilation is claimed.
