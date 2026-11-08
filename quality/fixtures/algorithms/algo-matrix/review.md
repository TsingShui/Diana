# Fixture: algo-matrix

matrix multiply, transpose and spiral traversal. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug algo-matrix`:
  structure 28%, naming 29%.

## Current verdict — round 1, 2026-11-08

### Semantics

The verdict is written by reading the reference next to the output. `score.py`
never substitutes for it.

| method | semantics |
|---|---|
| `multiply` | ~ 值正确；`int[][] out = new int[rows][columns]` 变成 `Array.newInstance(Integer.TYPE, …)`，且 `return v8;` 的静态类型是 `Object` |
| `transpose` | ~ 同上 |
| `spiral` | ~ 四边循环与边界更新可辨认，未逐行核对 |

### Score

| method | structure | naming |
|---|---|---|
| `multiply` | 24% | 18% |
| `transpose` | 50% | 25% |
| `spiral` | 11% | 43% |

### Findings

- 二维数组创建全部走 `Array.newInstance(Integer.TYPE, new int[]{…})`，之后读取要么带 cast 要么丢失静态类型（`return v8;`）
- 结构分最低：multiply 24%、spiral 11%，三重/四重循环都被展开成 `while` + 逐层临时
- `p1[0] /* Object load */` 之类注记遍布数组访问

### Ranked fixes

还原 `new int[m][n]` 与 `int[][]` 静态类型；数组读取不中转；循环嵌套形态恢复

## Rounds

- R1 2026-11-08: structure 28%, naming 29%; 二维数组创建全部走 `Array.newInstance(Integer.TYPE, new int[]{…})`，之后读取要么带 cast 要么丢失静态类型（`return v8;`）
