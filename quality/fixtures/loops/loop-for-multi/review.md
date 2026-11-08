# Fixture: loop-for-multi

for with several initialisers/updaters and empty segments. The Java source is the reference; the DEX is generated from it
with the fixture's `javac.args`/`d8.args` or the shared defaults (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/LoopForMulti;`

## Current verdict — round 1, 2026-11-08

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
