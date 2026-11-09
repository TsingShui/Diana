# Fixture: algo-graph

BFS, DFS and connected components over an adjacency matrix. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — iteration-tail review

Java and final text were reread for bfs(int[][],int), dfs(int[][],boolean[],int)
and components(int[][]). Neighbor updates follow the same tested edge, distance
write, boxing/enqueue or recursive count addition. A failed access/call does not
advance the counter. DFS's visited early return/write is unchanged; components
counts only after a successful DFS. The queue's outer while and all original
array/queue/call ordering remain intact.

This is direct LLM source/path evidence on the candidate based on `ebbc003`, not
executed behavioral PASS. Original P2/P3/P4/P5 and relevant P6 control/exception/
lock facts agree in corpus inspection. Unchanged member text reuses prior reviews.
The new [iteration-tail challenge](../../loops/iteration-tail-cleanup/review.md)
records the interacting cases and unsupported execution scope.

## Prior verdict — round 1, 2026-11-08

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
