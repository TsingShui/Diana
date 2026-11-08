# Fixture: algo-sort

insertion, quick and merge sort. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug algo-sort`:
  structure 50%, naming 48%.

## Current verdict — round 1, 2026-11-08

### Semantics

The verdict is written by reading the reference next to the output. `score.py`
never substitutes for it.

| method | semantics |
|---|---|
| `insertion` | ✔ 内层 `while (pos >= 0 && values[pos] > key)` 变成 `while` + `if (<=) break`，等价 |
| `quick` | ✔ 单参数入口正确转调私有重载 |
| `merge` | ✔ 三方归并、`<=` 选左、尾部拷贝与 `break` 均正确 |

### Score

| method | structure | naming |
|---|---|---|
| `insertion` | 60% | 33% |
| `quick` | 65% | 67% |
| `merge` | 26% | 45% |

### Findings

- `quick` 的分区循环用 `[repeat: …]` 相位注记表达 update，读起来要先解释注记
- `merge` 结构 26%：每个下标都先存临时再自增（`v14 = v25 + 1` 等），`i++` 形态全丢
- `insertion` 的 `for` 变成 `while (test { … })`

### Ranked fixes

循环形态与 `i++`/`+=` 恢复；相位注记只留给真正无法结构化的情况

## Rounds

- R1 2026-11-08: structure 50%, naming 48%; `quick` 的分区循环用 `[repeat: …]` 相位注记表达 update，读起来要先解释注记
