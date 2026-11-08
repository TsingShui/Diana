# Fixture: algo-graph

BFS, DFS and connected components over an adjacency matrix. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — round 1, 2026-11-08

### Semantics

The verdict is written by reading the reference next to the output.

| method | semantics |
|---|---|
| `bfs` | ✔ The raw ArrayDeque and Integer cast/unboxing after remove() preserve the values. |
| `dfs` | ✔ Visited nodes return 0; recursive accumulation over neighbors agrees. |
| `components` | ✔ Counting unvisited components agrees. |

### Findings

- `Deque<Integer> queue = new ArrayDeque<>()` becomes `ArrayDeque arrayDeque = new ArrayDeque()`, losing generic and interface declaration forms.
- `queue.remove()` is read as Object and cast; enqueueing exposes `Integer.valueOf`.
- Intermediate array-element locals and `while (test { ... })` obscure the structure. Recorded forms: `while (test { … })`.

### Ranked fixes

Preserve declared types and generic arguments; improve array reads and loop presentation.

## Rounds

- R1 2026-11-08: `Deque<Integer> queue = new ArrayDeque<>()` becomes `ArrayDeque arrayDeque = new ArrayDeque()`, losing generic and interface declaration forms.
