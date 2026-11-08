# Fixture: branch-if2

two-register branches. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug branch-if2`:
  structure 71%, naming 50%.

## Current verdict — round 1, 2026-11-08

### Per method

| method | structure | naming |
|---|---|---|
| `lessThan` | 69% | 50% |
| `greaterOrEqual` | 70% | 50% |
| `equal` | 74% | 50% |

### Semantics

The semantics verdict is written by reading the reference next to the output; `?` means not yet judged. `score.py` never substitutes for it.

| method | semantics |
|---|---|
| `lessThan` | ✔ 反演但等价 |
| `greaterOrEqual` | ✔ |
| `equal` | ✔ |

### Findings

- 三种比较都正确，但全部被反演/重排（`<` → `>=`、`else` 拆成早返回）
- 结构分 ~70%，命名分低是因为参数名 `left/right` 未恢复

### Ranked fixes

条件不反演；保持 `if/else` 而不是早返回

## Rounds

- R1 2026-11-08: structure 71%, naming 50%; 三种比较都正确，但全部被反演/重排（`<` → `>=`、`else` 拆成早返回）
