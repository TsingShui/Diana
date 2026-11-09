# Fixture: refined-carriers

## Current verdict — terminal guard follow-up

Read checkedEarly and tick: the original check stays before tick/selection;
negative mode returns0 before length. tick still marks before its return or ISE.
No cast is delayed or invocation/check origin duplicated.

Compared the same prepared DEX on 29f1b82 and this commit. P7 requires a complete
linear arm ending in a printed return/throw; opaque scopes are not exit evidence.
The [fresh guard challenge](../../branches/terminal-guard-arms/review.md) covers
actual other-edge Phi copies, parent scope, recovering catches and cleanup.
Original P2 through complete P6 products agree; this is bounded LLM path reading,
not behavioral execution. Historical execution claims below were not rerun.


## Prior LLM semantic judgement

Compared authored Java/Smali and original Engine products/source on the same
prepared DEX, completed-eleven-repair bda9a49 versus candidate. Independent
expectations were written after the general adjacency/order repair but before
this fixture's first inspection. No Java recompilation, scenario executor or
device execution was performed. This is bounded manual path reading.

| Original owner Lfixture/RefinedCarriers; subject | Reading |
|---|---|
| declared(Object)I | identity executes once, then the original check; initializer keeps the check's operation/PC. |
| checkedEarly(Object,int)I | Check precedes tick and early return even when no later read executes. |
| bytes()[B | Original allocation/exact fill remains byte[3] literal1,2,3 across adjacent portions. |
| guarded(Object)[B | Other-reference check stays between allocation and fill; no literal fusion crosses it. |
| identity(Object)Object, tick(II)V, mark(I)V | Same reference, arguments, wrapping trace and event-before-return/fault order. |

### Provenance and initializer

For declared("ok"), identity marks10 and returns the exact reference; the check
succeeds, then length returns2. An ordinary non-String object still marks10 but
fails its check before length. Null passes the check and faults at length after10.
The initializer performs one identity invocation and one check. Its InlineCheck
and source span name the CHECK operation/PC, not the preceding invocation. The
old record duplicated that invocation's anchor and omitted the check identity.

### Confirmed old ordering defect

The baseline printed tick(1,mode) before a cast at length and returned0 for
negative mode without executing that cast. A non-String input could wrongly
mark1, throw ISE at tick in mode1, or return0 in mode-1 instead of throwing CCE
before tick. Same storage identity and protection context were not an ordering
proof. Direct reading rejected that route.

Candidate retains the standalone check before tick/selection. Non-String inputs
throw CCE without events for modes0/1/-1. String "ok" retains2/0 after1 for
modes0/-1. Null's successful check still permits tick: mode0 faults at length,
mode1 throws ISE from tick, and mode-1 returns0, each after1. The check is not
removed merely because an early path never reads the value.

### Array placement

bytes's allocation and fill are adjacent in the selected same-original-block
schedule. Count/type/payload/storage requirements establish all three exact
elements with no intervening copy/event, preserving its literal. guarded instead
allocates, checks another input reference, then fills. A non-String input faults
after allocation and before fill; String/null succeeds with1,2,3. Shared adjacency
cannot skip that effectful portion even if a pass removes its separate action.
The negative is an actual different-reference check, not a payload/type refusal.

## Owning consumer changes

Whole-operation P6 portions exposed assumptions that adjacent actions shared one
node. One private P7 module consumes selected sibling Blocks: one within-block,
copy-free fallthrough, single arrival, no required label, continuation or opaque
scope crossing. Expression/check/array consumers share it and retain their own
phase/operand guards.

Check carrier priority is unchanged. Definition placement checks standalone
source and check phases/protection, using the check's identity. A later read
requires a continuous pure linear run with no effect, phase, selection or
protection change; target phases and suspension operands remain barriers.
No alternate analysis, graph/dominance reconstruction, external type facts or
production validation was introduced.

## Product evidence, not behavioral execution

Four ordinary-Engine product checks cover initializer/check origin, pre-tick
check, literal preservation and intervening-check barrier. The same prepared DEX
and tests also ran against isolated bda9a49 source: both array guards passed;
early-check and original-call-anchor checks failed as expected. All four pass
on candidate. This establishes product-check sensitivity to confirmed old defects;
it is NOT execution of the scenario matrix or behavioral PASS.

The [pure-tail challenge](../../exceptions/pure-tail-finally/review.md) supplies
the producer mechanism. Within recorded cases adjacent carriers are preserved
and delayed-check/origin defects repaired. Arbitrary inputs and README-excluded
faults/concurrency remain outside this judgement.

## Verification observations

Reproduce the product checks from the repository root with
`cargo nextest run --locked -p diana-decomp --test refined_carriers --no-fail-fast`.
Candidate is the commit containing this review; baseline is bda9a49 and both
consume the same committed classes.dex. Default and all-feature package suites
each ran 70 tests across 17 binaries with 70 passing. Package Clippy with warnings
denied, release/dbg_shell builds, format, diff and desensitization checks passed.

The corpus inspection observed 88 fixtures, 139 classes and 665 methods without
stopped members or render failures/issues. Original P2/P3/P4/P5 and P6 original
operation/edge/exception/lock/copy-phase projections agree despite changed portion
coordinates. Current changed paths have manual reviews or unchanged-source
binding correspondence; these observations do not execute the scenario matrix.
