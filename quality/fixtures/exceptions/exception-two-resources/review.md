# Fixture: exception-two-resources

try-with-resources over two resources. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Round-1 numbers from `python3 quality/score.py --slug exception-two-resources`:
  structure 8%, naming 33%.

## Current verdict — round 2

当前普通 Engine 输出已恢复嵌套保护层级。下表数值保留 R1 测量；本轮未重跑 score。

### Per method

| method | structure | naming | first divergence |
|---|---|---|---|
| `sum` | 8% | 33% | `insert: ref=- ours=ID ID ; ID ID` |

### Semantics

The semantics verdict is written by reading the reference next to the output; `?` means not yet judged. `score.py` never substitutes for it.

| method | semantics |
|---|---|
| `sum` | ? 已恢复双资源嵌套 try/catch；独立行为场景尚未执行 |

### Findings

- 第二资源的创建、读取与关闭位于外层 try；读取的 catch 内保留关闭第二资源及其 suppression 的内层 try/catch
- 第一资源的正常 close 保持在外层 try/catch 之后，失败不会重新进入同一清理路径
- handler 不再游离于 return 之后，正文不再需要 exception_chain、`[exn cN]` 或 goto
- 没有 `try (Res a = …; Res b = …)` 形态

### Ranked fixes

执行独立场景核对关闭和 suppressed 顺序；再评估资源声明形态与名字恢复

## Rounds

- R2: 双资源保护范围恢复为嵌套 try/catch；旧 ✘ 记录的是结构阅读障碍，并非已证实的执行不等价。当前行为验证保留 ?；未重测结构分。

- R1 2026-11-08: structure 8%, naming 33%; 结构 7.6% —— 语料里最差：handler `H6`/`H8` 打在 `return` 之后的方法层级，`try { H10:` 出现在 `throw v7` 之后
