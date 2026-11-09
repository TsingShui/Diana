# Fixture: loop-for-multi

## Current verdict — whole-operation follow-up

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
