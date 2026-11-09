# Fixture: loop-for-multi

## Current verdict — while/do-while architecture correction

Read all methods, including the first-test hazard: emptyCondition adds initial
step1 before comparing, returning1 for a negative limit. twoPointers retains
pair loads/stores, swaps++, left++, then right--; failures skip remaining
operations. No initializer or first addition moves into a header.

Compared the same prepared DEX on 8ac6d95 and this commit. Dedicated for recovery
contradicted the approved architecture and is removed, not retained behind a
switch. Updates stay in ordinary body positions with original targets/copies;
explicit gotos can be necessary instead of update-skipping while continues.
The [fresh challenge](../../loops/while-tail-continuations/review.md) covers
three-level private continuations, real shared entrances, cleanup and do-while.
This is bounded LLM source/path reading, not behavioral execution. Historical
execution claims below were not rerun and do not certify this output.


## Prior verdict — whole-operation follow-up

Read Java and complete output. twoPointers still loads/stores the selected pair,
increments swaps and left, then decrements right; an access failure skips
remaining work. endless/emptyUpdate preserve their original bounds and
increments. emptyCondition still adds initial step1 BEFORE its first comparison,
so a negative limit returns1; the update does not replace that first addition.

The [pure-tail challenge](../../exceptions/pure-tail-finally/review.md) and
[carrier challenge](../../atoms/refined-carriers/review.md) record the owning
producer/consumer repairs and blocked drafts. This is bounded current source
reading; historical execution claims below were not rerun and do not certify
this output. No behavioral executor, device run or Java recompilation ran.


for with several initialisers/updaters and empty segments. The Java source is the reference; the DEX is generated from it
with the fixture's `javac.args`/`d8.args` or the shared defaults (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/LoopForMulti;`

## Prior verdict — iteration-tail review

Java and final text were reread. Only emptyCondition(I)I changes: for(;;step++)
still executes index+=step BEFORE its explicit index>limit break, including its
first test. A negative limit therefore returns 1, not 0; a break skips step++.
Other loop forms and multiple-updater behavior are unchanged. The supplementary
product/source-anchor check preserves the original pre-test assignment.

This is direct LLM source/path evidence on the candidate based on `ebbc003`, not
executed behavioral PASS. Original P2/P3/P4/P5 and relevant P6 control/exception/
lock facts agree in corpus inspection. Unchanged member text reuses prior reviews.
The new [iteration-tail challenge](../../loops/iteration-tail-cleanup/review.md)
records the interacting cases and unsupported execution scope.

## Prior verdict — round 1, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `twoPointers` | ✔ Two-pointer selection and swaps agree; both updates remain at the loop tail. |
| `endless` | ✔ The endless for plus break folds into while(index < limit). |
| `emptyUpdate` | ✔ The empty-update for body and update agree in the recovered loop body. |
| `emptyCondition` | ✔ The body moves into the predicate block, retaining the intended values. |

### Findings

- Multiple initializers/updates become separate assignments, obscuring the for structure.
- Endless/partial for loops become while; emptyCondition moves its body into a test block.
- All four methods use while presentation rather than their original for forms.

### Ranked fixes

Improve for headers, multiple initializers and preservation of empty header sections.

## Rounds

- R1 2026-11-08: Multiple initializers/updates become separate assignments, obscuring the for structure.
