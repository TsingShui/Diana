# Fixture: exception-finally-transfer

finally with return/break/continue, and try inside catch. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/ExceptionFinallyTransfer;`
- Current numbers from `python3 quality/score.py --slug exception-finally-transfer`:
  structure 49%.

## Current verdict — round 1, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `returnInFinally` | ✔ finally 里的 `return -1` 覆盖了 try 的返回值（与 Java 语义一致） |
| `breakInFinally` | ✔ finally 里的 `break` 丢弃挂起异常并退出循环，异常边同样落 `L6: state=3` |
| `continueInFinally` | ✔ finally 在 `continue` 与正常路径都执行 |
| `tryInsideCatch` | ✔ catch 内再 try，两层 `exception_chain` 与两个 catch 都对 |

### Score

| method | structure |
|---|---|
| `returnInFinally` | 59% |
| `breakInFinally` | 43% |
| `continueInFinally` | 46% |
| `tryInsideCatch` | 49% |

### Findings

- `finally` 里的 `return`/`break`/`continue` 全部被展开成 goto + 复制体，没有 `finally` 关键字
- `breakInFinally` 的异常边 `H10` 会**落入** `L6: state=3` 后 `return`——这正是 Java 的语义（break 吞掉异常），但文本上看像错误
- 每个操作仍挂 `[exn c1]`；`R9:`/`R7:` 之类区间标记与 `exception_chain` 混排

### Ranked fixes

把 break/continue/return 的 finally 复制体收敛回 `finally`；区间标记移出正文

## Rounds

- R1 2026-11-08: structure 49%; `finally` 里的 `return`/`break`/`continue` 全部被展开成 goto + 复制体，没有 `finally` 关键字
