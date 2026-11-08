# Fixture: algo-graph

BFS, DFS and connected components over an adjacency matrix. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug algo-graph`:
  structure 50%, naming 45%.

## Current verdict — round 1, 2026-11-08

### Semantics

The verdict is written by reading the reference next to the output. `score.py`
never substitutes for it.

| method | semantics |
|---|---|
| `bfs` | ✔ `ArrayDeque` 原始类型、`remove()` 后 `((Integer) v7).intValue()`，取值正确 |
| `dfs` | ✔ 访问过的节点返回 0、邻居递归累加正确 |
| `components` | ✔ 未访问节点计数正确 |

### Score

| method | structure | naming |
|---|---|---|
| `bfs` | 27% | 56% |
| `dfs` | 62% | 29% |
| `components` | 62% | 50% |

### Findings

- `Deque<Integer> queue = new ArrayDeque<>()` 印成 `ArrayDeque arrayDeque = new ArrayDeque()`，泛型与接口类型都丢失
- `queue.remove()` 的返回值被当作 `Object` 再 cast，`Integer.valueOf` 出现在入队处
- 结构分低（27%~63%）主要来自数组元素的中转临时与 `while (test { … })`

### Ranked fixes

保留声明类型与泛型实参；数组读取不中转临时；循环形态恢复

## Rounds

- R1 2026-11-08: structure 50%, naming 45%; `Deque<Integer> queue = new ArrayDeque<>()` 印成 `ArrayDeque arrayDeque = new ArrayDeque()`，泛型与接口类型都丢失
