# Fixture: loop-shared-exit

nested loops leaving through a flag or a labelled break. The Java source is the reference; the DEX is generated from it
with the fixture's `javac.args`/`d8.args` or the shared defaults (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/LoopSharedExit;`
- Current numbers from `python3 quality/score.py --binary target/debug/diana --slug loop-shared-exit`:
  structure 49.4%, naming 31.2%.

## Current verdict — round 2, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `flagExit` | ✔ 命中处理在内层分支执行一次，随后 break；正常耗尽不执行命中处理，外层更新仍执行 |
| `labelledExit` | ✔ 命中直接 return；负数分支 break R29 退出外层，跳过两层更新并返回 -1 |
| `sharedTail` | ✔ 第一轮跳过负数、第二轮跳过正数，两轮 continue 路径均保留各自的索引更新 |

### Per method

| method | structure | naming |
|---|---|---|
| `flagExit` | 53.2% | 25% |
| `labelledExit` | 53.1% | 29% |
| `sharedTail` | 42.0% | 40% |

### Findings

- for (… && !found; …) 的布尔条件被拆到循环体里（if (v12 != 0) break;），found 标志物化成 int
- 内层命中处理已收进分支，用 break 退出内层；正常出口的 planned continuation goto 同时消失
- 负数分支直接打印带标签的 break；这两个方法原来的四处 goto 全部消失
- 本轮只改变 flagExit 和 labelledExit；sharedTail 输出保持不变

### Ranked fixes

布尔标志恢复 boolean；条件中的数组读取合成与多余临时变量仍影响可读性

## Rounds
- R3 2026-11-08: 空分支清理；`else { }` 与空 `if` 不再打印，空真分支反演成单臂 if（谓词求值位置不变）

- R1 2026-11-08: structure 47%; for (… && !found; …) 的布尔条件被拆到循环体里（if (v12 != 0) break;），found 标志物化成 int
- R2 2026-11-08: structure 49.4%, naming 31.2%; 嵌套私有退出路径保留在循环体，四处 goto 改为 break/带标签的 break；逐方法语义复读
