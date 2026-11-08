# Fixture: loop-for

classic for, with break and continue in the body. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug loop-for`:
  structure 62%, naming 38%.

## Current verdict — round 1, 2026-11-08

### Per method

| method | structure | naming | first divergence |
|---|---|---|---|
| `sumTo` | 70% | 25% | `delete: ref== 0 ours=-` |
| `skipNegative` | 50% | 40% | `delete: ref== 0 ours=-` |
| `stopAt` | 66% | 50% | `delete: ref=for ( ours=-` |

### Semantics

The semantics verdict is written by reading the reference next to the output; `?` means not yet judged. `score.py` never substitutes for it.

| method | semantics |
|---|---|
| `sumTo` | ✔ |
| `skipNegative` | ✔ continue 变空 else，等价 |
| `stopAt` | ✔ break 保留 |

### Findings

- 三个 `for` 全部没有还原（首个差异就是 `for (` 缺失），变成 `while` + 循环前置赋值
- `continue`/`break` 仍是标签 + `goto`（`skipNegative` 结构 50%）
- 取值与累加语义保留

### Ranked fixes

还原 `for (init; cond; update)`；`break`/`continue` 直接印出；参数名恢复

## Rounds

- R1 2026-11-08: structure 62%, naming 38%; 三个 `for` 全部没有还原（首个差异就是 `for (` 缺失），变成 `while` + 循环前置赋值
