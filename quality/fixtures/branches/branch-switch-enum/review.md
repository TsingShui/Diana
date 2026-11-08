# Fixture: branch-switch-enum

switch over an enum, including a default arm. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug branch-switch-enum`:
  structure 42%, naming 25%.

## Current verdict — round 1, 2026-11-08

### Per method

| method | structure | naming | first divergence |
|---|---|---|---|
| `apply` | 42% | 25% | `insert: ref=- ours=int ID = ID .` |

### Semantics

The semantics verdict is written by reading the reference next to the output; `?` means not yet judged. `score.py` never substitutes for it.

| method | semantics |
|---|---|
| `apply` | ✔ case 0/1/2 对应 FAST/SAFE/LEGACY |

### Findings

- `apply` 用 `int v = p0.ordinal()` 直接 switch，`$SwitchMap` 被消解（这是好事）
- 结构 42% 的差距来自临时变量、`default:` 位置与 `case` 顺序
- 取值正确

### Ranked fixes

enum switch 保持 `switch (mode)` 语义与 case 顺序；`default` 最后；参数名恢复

## Rounds

- R1 2026-11-08: structure 42%, naming 25%; `apply` 用 `int v = p0.ordinal()` 直接 switch，`$SwitchMap` 被消解（这是好事）
