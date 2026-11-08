# Fixture: algo-dp

edit distance, longest increasing subsequence and knapsack. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug algo-dp`:
  structure 14%, naming 37%.

## Current verdict — round 2, 2026-11-08

### Semantics

The verdict is written by reading the reference next to the output. `score.py`
never substitutes for it.

| method | semantics |
|---|---|
| `editDistance` | ✔ 边界初始化、`Math.min` 三层与对角线递归都正确；2D 数组现声明为 `int[][]`，执行对比 64 组一致 |
| `longestIncreasing` | ✔ 双重循环与 `best[i] = best[j] + 1` 正确 |
| `knapsack` | ✔ 逆序容量循环与候选比较正确 |

### Score

| method | structure | naming |
|---|---|---|
| `editDistance` | 14% | 50% |
| `longestIncreasing` | 9% | 40% |
| `knapsack` | 64% | 22% |

### Findings

- `new int[m][n]` 被解成 `Array.newInstance(Integer.TYPE, new int[]{m,n})` 加 `Object` 变量，多数读取带 `/* Object load */` 且缺 cast
- 结构分最低的一批（editDistance 14%、longestIncreasing 8%），全部来自中转临时与循环形态
- `Math.min(v32, v35)` 与 `Math.min(v29, v37)` 的嵌套顺序正确

### Ranked fixes

`new int[m][n]` 的原样还原；数组读取不中转；循环形态恢复

## Rounds

- R2 2026-11-08: structure 14%, naming 37%（同一二进制下的重测值；R1 的 29% 来自更早的构建）；2D 数组声明恢复为 `int[][]`，三个方法通过执行对比（1646 组）
- R1 2026-11-08: structure 29%, naming 37%; `new int[m][n]` 被解成 `Array.newInstance(Integer.TYPE, new int[]{m,n})` 加 `Object` 变量，多数读取带 `/* Object load */` 且缺 cast
