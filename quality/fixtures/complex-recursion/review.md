# Fixture: complex-recursion

mutual recursion, accumulator recursion and memoisation. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug complex-recursion`:
  structure 66%, naming 55%.

## Current verdict — round 1, 2026-11-08

### Semantics

The verdict is written by reading the reference next to the output. `score.py`
never substitutes for it.

| method | semantics |
|---|---|
| `isEven` | ✔ 反演但等价（`if (p0 != 0) return isOdd(p0-1)`） |
| `isOdd` | ✔ |
| `sumTo` | ✔ 入口转调私有重载 |
| `fibonacci` | ✔ 记忆化：先查 `memo`、缺失时递归并写回 |
| `factorial` | ✔ 迭代连乘 |

### Score

| method | structure | naming |
|---|---|---|
| `isEven` | 62% | 67% |
| `isOdd` | 62% | 67% |
| `sumTo` | 95% | 50% |
| `fibonacci` | 48% | 67% |
| `factorial` | 63% | 25% |

### Findings

- 互递归与累加器递归都正确；`fibonacci` 的记忆化读/写顺序保持
- `Map<Integer, Long> memo` 的声明保留了泛型，但 `this.memo = hashMap` 的右侧是原始 `HashMap`
- 结构 48%~95%，扣分主要来自临时变量与 `while` 形态

### Ranked fixes

字段声明与初始化右侧的泛型一致；临时变量收敛

## Rounds

- R1 2026-11-08: structure 66%, naming 55%; 互递归与累加器递归都正确；`fibonacci` 的记忆化读/写顺序保持
