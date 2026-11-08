# Fixture: branch-ifz

single-register branches. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug branch-ifz`:
  structure 89%, naming 70%.

## Current verdict — round 1, 2026-11-08

### Per method

| method | structure | naming |
|---|---|---|
| `whenZero` | 96% | 67% |
| `whenNonZero` | 96% | 67% |
| `bothPaths` | 68% | 67% |
| `nullCheck` | 96% | 80% |

### Semantics

The semantics verdict is written by reading the reference next to the output; `?` means not yet judged. `score.py` never substitutes for it.

| method | semantics |
|---|---|
| `whenZero` | ✔ 分支未反演 |
| `whenNonZero` | ✔ 分支未反演 |
| `bothPaths` | ✔ 分支未反演 |
| `nullCheck` | ✔ 分支未反演 |

### Findings

- 条件形状全部保留（`if (p0 == 0)`、`if (p0 != 0)`、`if (input == null)` 都没有被反演）
- 扣分只来自命名（`p0`）与字段限定 `BranchIfz.state`，不是结构

### Ranked fixes

参数名从 debug_info 恢复；字段限定在无冲突时省略

## Rounds

- R1 2026-11-08: structure 89%, naming 70%; 条件形状全部保留（`if (p0 == 0)`、`if (p0 != 0)`、`if (input == null)` 都没有被反演）
