# Fixture: exception-finally

finally on the normal path and on the throwing path. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug exception-finally`:
  structure 78%, naming 50%.

## Current verdict — round 1, 2026-11-08

### Per method

| method | structure | naming | first divergence |
|---|---|---|---|
| `normalPath` | 70% | 50% | `delete: ref=0 ; try { ID ours=-` |
| `throwingPath` | 86% | 50% | `delete: ref== 0 ours=-` |

### Semantics

The semantics verdict is written by reading the reference next to the output; `?` means not yet judged. `score.py` never substitutes for it.

| method | semantics |
|---|---|
| `normalPath` | ✔ finally 折进正常路径，等价 |
| `throwingPath` | ✔ finally 恢复为 finally 块 |

### Findings

- `finally` 被拆成正常路径重复 + `catch (all)` 重抛（与之前一致）
- `throwingPath` 结构 86%：`return acc` 被移入正常路径，`finally` 的字段写在两条路径上各一次
- 语义正确：`state` 在正常与异常路径都被写

### Ranked fixes

识别 `finally` 而不是 `catch (all)` + 重抛；`int acc = 0;` 不再前置成临时；参数名恢复

## Rounds

- R1 2026-11-08: structure 78%, naming 50%; `finally` 被拆成正常路径重复 + `catch (all)` 重抛（与之前一致）
