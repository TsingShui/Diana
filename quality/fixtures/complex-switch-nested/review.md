# Fixture: complex-switch-nested

switch inside a loop inside try, with labelled break and continue. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug complex-switch-nested`:
  structure 17%, naming 38%.

## Current verdict — round 1, 2026-11-08

### Semantics

The verdict is written by reading the reference next to the output. `score.py`
never substitutes for it.

| method | semantics |
|---|---|
| `run` | ~ 标签跳转与 `[after_operands_before_effect]` 快照交错，无法仅凭文本确认 `continue outer`/`break outer` 的落点 |

### Score

| method | structure | naming |
|---|---|---|
| `run` | 17% | 38% |

### Findings

- `outer:` 标签被折叠成 `goto L13/L16/L17/L19`，`continue outer` 与 `break outer` 的落点无法从文本确认
- 每个操作都挂 `[after_operands_before_effect {v22 = v19;}]` 快照，readability 崩掉（结构 17%）
- 内层循环里出现空 `if (v7 != v5) { }`

### Ranked fixes

先让带标签的 break/continue 有可读形式；快照注记只保留真正需要的边；空 if 消除

## Rounds

- R1 2026-11-08: structure 17%, naming 38%; `outer:` 标签被折叠成 `goto L13/L16/L17/L19`，`continue outer` 与 `break outer` 的落点无法从文本确认
