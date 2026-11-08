# Fixture: complex-exceptions

nested try with finally, caught rethrow with cause, TWR in a loop, multi-catch. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug complex-exceptions`:
  structure 30%, naming 63%.

## Current verdict — round 1, 2026-11-08

### Semantics

The verdict is written by reading the reference next to the output. `score.py`
never substitutes for it.

| method | semantics |
|---|---|
| `parseOrThrow` | ✔ 语义正确，但构造器调用印成 `new IllegalArgumentException;` + `e.<init>(msg, cause)` |
| `nestedFinally` | ✔ 内层 finally 在正常/异常两条路径都执行，外层 finally 同样，catch 返回 -1 |
| `resources` | ✘ TWR 在循环里：`H6` 打在 `goto L10` 之后、`try { H6: … }` 出现在 `throw` 之后，保护范围不可跟踪 |
| `multiAndInstance` | ✔ `instanceof`/cast/两个 catch 与 return -2/-3 正确；`via` 与悬空 cast 语句是记号问题 |

### Score

| method | structure | naming |
|---|---|---|
| `parseOrThrow` | 43% | 82% |
| `nestedFinally` | 11% | 57% |
| `resources` | 46% | 29% |
| `multiAndInstance` | 20% | 86% |

### Findings

- `resources` 与 `exception-two-resources` 是同一个缺陷：handler 打在保护范围外
- 构造器调用被拆成 `new X;` + `x.<init>(…)`，正文里出现 `IllegalArgumentException illegalArgumentException = new IllegalArgumentException;`
- `(v3 via Number)` 与 `((Number) v3) [exn c0];` 这类记号进入正文
- `log.add` 前的 `List log = this.log` 每次重新取字段

### Ranked fixes

先修 handler 归属（两处同因）；再合并 `new X(...)` 并去掉 `via`/悬空 cast；字段读取不中转局部

## Rounds

- R1 2026-11-08: structure 30%, naming 63%; `resources` 与 `exception-two-resources` 是同一个缺陷：handler 打在保护范围外
