# Fixture: algo-sort

insertion, quick and merge sort. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug algo-sort`:
  structure 29%, naming 31%.

## Current verdict — round 2, 2026-11-08

### Semantics

The verdict is written by reading the reference next to the output. `score.py`
never substitutes for it.

| method | semantics |
|---|---|
| `insertion` | ✔ 内层 `while (pos >= 0 && values[pos] > key)` 变成 `while` + `if (<=) break`，等价 |
| `quick` | ✔ 单参数入口正确转调私有重载；分区循环与前缀递归顺序正确 |
| `merge` | ✔ 三方归并、`<=` 选左、尾部拷贝与 `break` 均正确 |

### Score

| method | structure | naming |
|---|---|---|
| `insertion` | 46% | 33% |
| `quick` | 21% | 33% / 12% |
| `merge` | 26% | 45% |

### Findings

- 分区循环的 `[repeat: …]` 相位注记消失：`while (v16 <= v18)` 直接印出，`v17`/`v19` 合并进 `v16`/`v18`
- 剩余差距来自显示形态而不是循环：`int v6 = p1;` 之后再 `v16 = v6`、每个下标先存临时再自增、`for` 未还原
- `quick` 的私有重载与公开入口同名，`score.py` 的报告顺序按方法名排序

### Independent execution

`quick` 按原方法与渲染方法对比执行 896 组（数组内容 × 两个下标）：返回值、数组原地修改、异常类型一致，0 处不等价。校验器对被改坏的交换语句立即报出差异。这是一次有界输入对比，不是全输入证明。

### Ranked fixes

`i++`/`+=` 与 `for` 的显示形态；私有重载的声明顺序

## Rounds

- R1 2026-11-08: structure 50%, naming 48%; `quick` 的分区循环用 `[repeat: …]` 相位注记表达 update，读起来要先解释注记
- R2 2026-11-08: structure 29%, naming 31%（同一二进制下的重测值；R1 的 50%/48% 来自更早的构建）；相位注记消失，相差集中在临时变量与 `for` 形态
