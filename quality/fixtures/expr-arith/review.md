# Fixture: expr-arith

arithmetic, bits, shifts. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug expr-arith`:
  structure 86%, naming 44%.

## Current verdict — round 1, 2026-11-08

### Per method

| method | structure | naming |
|---|---|---|
| `add` | 95% | 33% |
| `minusOne` | 91% | 50% |
| `multiply` | 72% | 50% |
| `divide` | 95% | 33% |
| `modulo` | 94% | 50% |
| `bits` | 75% | 33% |
| `shifts` | 74% | 50% |
| `negate` | 93% | 50% |

### Findings

- 算术结果正确；主要差异是每个表达式被拆成 `int ID = (…)` 再 `return`
- `x * 17L` 的 wide 乘法被拆成两行（结构 72%）

### Ranked fixes

单表达式直接 `return x + 1;`

## Rounds

- R1 2026-11-08: structure 86%, naming 44%; 算术结果正确；主要差异是每个表达式被拆成 `int ID = (…)` 再 `return`
