# Fixture: complex-lambda

lambdas, capturing lambda, anonymous supplier, method reference and comparator. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug complex-lambda`:
  structure 64%, naming 57%.

## Current verdict — round 1, 2026-11-08

### Semantics

The verdict is written by reading the reference next to the output. `score.py`
never substitutes for it.

| method | semantics |
|---|---|
| `printer` | ✔ 捕获通过 `ComplexLambda$0(this, p0)`，lambda 体在 `lambda$printer$0$fixture-ComplexLambda` |
| `counter` | ✔ 匿名类 `ComplexLambda$1` 带状态 |
| `applyTwice` | ✔ |
| `sorted` | ? 未读 |
| `describe` | ✔ `append("value:").append(p0)` 等价 |
| `prefixer` | ✔ 方法引用变成 `ComplexLambda$2` |

### Score

| method | structure | naming |
|---|---|---|
| `printer` | 65% | 38% |
| `counter` | 56% | 43% |
| `applyTwice` | 61% | 43% |
| `sorted` | 79% | 78% |
| `describe` | 54% | 67% |
| `prefixer` | 68% | 75% |

### Findings

- lambda 体方法的真实名字是 `lambda$printer$0$fixture-ComplexLambda`，加上 `dex_flags<synthetic>` 进正文
- 类体里出现 R8 合成名 `$r8$lambda$ey1lUbfxWqoov2vHAeHNf7_uUW8`（方法引用的实现体）
- `sorted` 的 `Comparator.naturalOrder()` 未核实，结构 79%

### Ranked fixes

lambda 体与其合成名不出正文；`dex_flags<>` 移出；恢复 `list.sort(...)` 形态

## Rounds

- R1 2026-11-08: structure 64%, naming 57%; lambda 体方法的真实名字是 `lambda$printer$0$fixture-ComplexLambda`，加上 `dex_flags<synthetic>` 进正文
