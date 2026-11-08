# Fixture: expr-compare

comparisons and boolean composition. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug expr-compare`:
  structure 53%, naming 38%.

## Current verdict — round 1, 2026-11-08

### Per method

| method | structure | naming |
|---|---|---|
| `lessThan` | 48% | 33% |
| `atLeast` | 48% | 33% |
| `same` | 51% | 50% |
| `different` | 57% | 33% |
| `both` | 43% | 33% |
| `either` | 37% | 33% |
| `negated` | 84% | 50% |

### Findings

- 比较结果正确，但 `return a < b` 变成 `int v; if (a < b) …`（结构 ~48%）
- `!value`、`a && b` 同样被展开成临时变量 + if

### Ranked fixes

布尔表达式直接返回谓词

## Rounds

- R1 2026-11-08: structure 53%, naming 38%; 比较结果正确，但 `return a < b` 变成 `int v; if (a < b) …`（结构 ~48%）
