# Fixture: algo-tree

binary search tree: recursive insert/height/inOrder and iterative contains. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — round 1, 2026-11-08

### Semantics

The verdict is written by reading the reference next to the output.

| method | semantics |
|---|---|
| `insert` | ✔ Recursive insertion and the root update agree. |
| `contains` | ✔ Inverted comparisons preserve branch selection. |
| `height` | ✔ The maximum of both recursive heights plus one agrees. |
| `inOrder` | ✔ The output list is filled left-root-right. |

### Findings

- Methods are presented by name, interleaving insert/height/inOrder overloads with their entries; use full prototypes to distinguish them.
- `List<Integer> out = new ArrayList<>()` becomes a raw ArrayList declaration. Recorded forms: `ArrayList arrayList = new ArrayList()`.
- `Integer.valueOf(p0.key)` precedes add; the value agrees but boxing is explicit.
- Field reads become intermediate locals, such as `AlgoTree$Node v2 = p0.left; this.height(v2)`.

### Ranked fixes

Improve field-read composition and generic arguments; distinguish overloads by full prototype.

## Rounds

- R1 2026-11-08: Methods are presented by name, interleaving insert/height/inOrder overloads with their entries; use full prototypes to distinguish them.
