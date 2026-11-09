# Carriers across whole-operation portions

## Independent expectations

Authored AFTER the shared P7 adjacency and cast-order repairs, BEFORE first
inspection. Discovery: canonical P6 operation portions needed by
[pure-tail-finally](../../exceptions/pure-tail-finally/README.md). This exposed
consumers that assumed multiple operations shared one portion: declaration
checks and array literals lost their adjacent carriers, while an unbounded
distant check search could delay a check past a call/branch.

Owner Lfixture/RefinedCarriers;. Subjects declared(Object)I,
checkedEarly(Object,int)I, bytes()[B, guarded(Object)[B, identity(Object)Object,
tick(II)V and mark(I)V. Java explains the actual DEX; an implicit constructor is
not a Smali subject. identity marks10 then returns its exact input reference.
tick marks its event then mode1 throws ISE("tick"). mark wraps trace*31+event;
trace starts0. String literals/model objects below are supplied caller values.
Array allocation and ordinary constructors succeed; no asynchronous/linkage/
allocation fault or concurrent trace mutation is modeled.

| Subject/arguments | Expected exit | Ordered mark arguments |
|---|---|---|
| declared("ok") | return2 | 10 |
| declared(ordinary non-String Object) | ClassCastException | 10 |
| declared(null) | NPE at length after successful null cast | 10 |
| checkedEarly("ok",0) | return2 | 1 |
| checkedEarly("ok",-1) | return0 | 1 |
| checkedEarly(non-String Object,0), (...,1), (...,-1) | ClassCastException BEFORE tick | none |
| checkedEarly(null,0) | NPE at length | 1 |
| checkedEarly(null,1) | ISE at tick | 1 |
| checkedEarly(null,-1) | return0 | 1 |
| bytes() | fresh byte[3] containing1,2,3 | none |
| guarded("ok") or guarded(null) | fresh byte[3] containing1,2,3 | none |
| guarded(non-String Object) | ClassCastException after allocation, BEFORE payload fill | none |

declared must attach the CHECK's operation/PC origin to the initializer, not
duplicate the preceding identity invocation's origin. checkedEarly must perform
its check even when mode<0 returns without reading value. A later matching read
or the same natural protection context alone is not an ordering guarantee.

bytes is a positive whole-payload literal case across adjacent operation portions.
guarded has a different-reference check between allocation and fill. It must not
be fused across that effect, even if the check action becomes an inline carrier.
No Java legality or recompilation is required, and no behavioral executor ran.

## Preparation

Assemble once using the repository Smali procedure, API24, into the ignored
fixture cache and copy only classes.dex here. Compare the same DEX on completed11
(commit bda9a49) and candidate through ordinary dbg_shell. Prepared using
Smali2.5.2, API24, OpenJDK21.0.12.1. Generated cache output remains ignored; only
classes.dex is committed here. The same-DEX comparison and [manual review](review.md)
are complete. Four product checks pass on candidate; baseline fails early-check
and invocation-anchor checks and passes both array guards. These are product
observations, not execution of the independent scenarios.
