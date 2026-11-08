# Fixture: array-basic

array creation/read/write. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug array-basic`:
  structure 85%, naming 60%.

## Current verdict — round 1, 2026-11-08

### Per method

| method | structure | naming |
|---|---|---|
| `sized` | 80% | 50% |
| `filled` | 84% | 100% |
| `bytes` | 83% | 100% |
| `flags` | 83% | 100% |
| `read` | 85% | 33% |
| `write` | 100% | 25% |
| `length` | 82% | 50% |
| `nested` | 80% | 25% |

### Semantics

The semantics verdict is written by reading the reference next to the output; `?` means not yet judged. `score.py` never substitutes for it.

| method | semantics |
|---|---|
| `sized` | ✔ 取值与顺序正确 |
| `filled` | ✔ 取值与顺序正确 |
| `bytes` | ✔ 取值与顺序正确 |
| `flags` | ✔ 取值与顺序正确 |
| `read` | ✔ 取值与顺序正确 |
| `write` | ✔ 取值与顺序正确 |
| `length` | ✔ 取值与顺序正确 |
| `nested` | ✔ 取值与顺序正确 |

### Findings

- 创建、读写、`.length`、二维 `matrix[row][column]` 都正确
- 数组字面量（`filled`/`bytes`/`flags`）与 `new int[n]` 都被拆成临时变量再 `return`（结构 85%、命名 60%）

### Ranked fixes

数组创建/字面量直接内联到 return；参数名恢复

## Rounds

- R1 2026-11-08: structure 85%, naming 60%; 创建、读写、`.length`、二维 `matrix[row][column]` 都正确
