# Fixture: complex-exceptions

nested try with finally, caught rethrow with cause, TWR in a loop, multi-catch. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Round-1 numbers from `python3 quality/score.py --slug complex-exceptions`:
  structure 30%, naming 63%.

## Current verdict — round 2

当前普通 Engine 输出已恢复循环内资源清理的保护层级。下表数值为 R1；本轮未重跑 score。

### Semantics

The verdict is written by reading the reference next to the output. `score.py`
never substitutes for it.

| method | semantics |
|---|---|
| `parseOrThrow` | ✔ 语义正确，但构造器调用印成 `new IllegalArgumentException;` + `e.<init>(msg, cause)` |
| `nestedFinally` | ✔ 内层 finally 在正常/异常两条路径都执行，外层 finally 同样，catch 返回 -1 |
| `resources` | ? 资源读取及其清理 handler 已同属循环内 try/catch；独立行为场景尚未执行 |
| `multiAndInstance` | ✔ `instanceof`/cast/两个 catch 与 return -2/-3 正确；`via` 与悬空 cast 语句是记号问题 |

### Score

| method | structure | naming |
|---|---|---|
| `parseOrThrow` | 43% | 82% |
| `nestedFinally` | 11% | 57% |
| `resources` | 46% | 29% |
| `multiAndInstance` | 20% | 86% |

### Findings

- `resources` 的私有 handler 现在属于循环词法体，包含 close/suppression 的内层 try/catch；正常 close 在读取保护之外，正文不再依赖游离 handler 或显式异常链
- `nestedFinally` 的内层保护已恢复，外层混合 catch/finally 仍使用显式异常链
- 构造器调用被拆成 `new X;` + `x.<init>(…)`，正文里出现 `IllegalArgumentException illegalArgumentException = new IllegalArgumentException;`
- `(v3 via Number)` 与 `((Number) v3) [exn c0];` 这类记号进入正文
- `log.add` 前的 `List log = this.log` 每次重新取字段

### Ranked fixes

执行资源关闭/异常顺序的独立场景；继续核对混合 finally 结构；保留构造与运行时 cast 的原始顺序

## Rounds

- R2: `resources` 恢复循环内嵌套 try/catch；旧 ✘ 的结构障碍已消除，行为验证仍为 ?，未重测结构分。

- R1 2026-11-08: structure 30%, naming 63%; `resources` 与 `exception-two-resources` 是同一个缺陷：handler 打在保护范围外
