# Fixture: exception-two-resources

try-with-resources over two resources. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug exception-two-resources`:
  structure 8%, naming 33%.

## Current verdict — round 1, 2026-11-08

### Per method

| method | structure | naming | first divergence |
|---|---|---|---|
| `sum` | 8% | 33% | `insert: ref=- ours=ID ID ; ID ID` |

### Semantics

The semantics verdict is written by reading the reference next to the output; `?` means not yet judged. `score.py` never substitutes for it.

| method | semantics |
|---|---|
| `sum` | ✘ 打印结构不可跟踪（handler 在 return 之后、try 出现在 throw 之后）；意图正确 |

### Findings

- 结构 7.6% —— 语料里最差：handler `H6`/`H8` 打在 `return` 之后的方法层级，`try { H10:` 出现在 `throw v7` 之后
- 意图（`close`、`addSuppressed`、按逆序关闭）都在，但打印出来的保护范围不可跟踪
- 没有 `try (Res a = …; Res b = …)` 形态

### Ranked fixes

两资源 TWR 的 handler 必须落在其保护范围内；再谈 `try (` 形态；名字恢复

## Rounds

- R1 2026-11-08: structure 8%, naming 33%; 结构 7.6% —— 语料里最差：handler `H6`/`H8` 打在 `return` 之后的方法层级，`try { H10:` 出现在 `throw v7` 之后
