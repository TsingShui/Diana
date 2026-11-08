# Fixture: algo-matrix

matrix multiply, transpose and spiral traversal. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug algo-matrix`:
  structure 28%, naming 29%.

## Current verdict — round 2, 2026-11-08

### Semantics

The verdict is written by reading the reference next to the output. `score.py`
never substitutes for it.

| method | semantics |
|---|---|
| `multiply` | ✔ 声明为 `int[][]` 并在初始化处带一次 cast；执行对比 64 组一致 |
| `transpose` | ✔ 同上；执行对比 8 组一致 |
| `spiral` | ✔ 四边循环与边界更新正确；执行对比 8 组一致 |

### Score

| method | structure | naming |
|---|---|---|
| `multiply` | 24% | 18% |
| `transpose` | 50% | 25% |
| `spiral` | 11% | 43% |

### Findings

- 二维数组仍走 `Array.newInstance(Integer.TYPE, …)`，但已声明成 `int[][] v = (int[][]) …`，`return v;` 与 `v[i]` 全部类型正确，类文本可直接编译
- 三个方法进入执行对比（multiply 64、transpose 8、spiral 8 组，共 80 组，0 处不等价）
- 剩余差距是形态而非类型：三重/四重循环展开成 `while` + 逐层临时，`p1[0] /* Object load */` 注记仍在

### Ranked fixes

还原 `new int[m][n]` 与 `int[][]` 静态类型；数组读取不中转；循环嵌套形态恢复

## Rounds

- R2 2026-11-08: structure 28%, naming 29%; 二维数组声明恢复为 `int[][]`，检查折到初始化处，三个方法通过执行对比
- R1 2026-11-08: structure 28%, naming 29%; 二维数组创建全部走 `Array.newInstance(Integer.TYPE, new int[]{…})`，之后读取要么带 cast 要么丢失静态类型（`return v8;`）
