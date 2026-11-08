# Fixture: expr-constants

constants only. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug expr-constants`:
  structure 96%, naming 96%.

## Current verdict — round 1, 2026-11-08

### Per method

| method | structure | naming |
|---|---|---|
| `intValue` | 100% | 100% |
| `longValue` | 100% | 100% |
| `floatValue` | 100% | 100% |
| `doubleValue` | 96% | 100% |
| `charValue` | 91% | 100% |
| `booleanValue` | 100% | 100% |
| `stringValue` | 100% | 67% |
| `nullValue` | 100% | 100% |
| `classValue` | 81% | 100% |

### Semantics

The semantics verdict is written by reading the reference next to the output; `?` means not yet judged. `score.py` never substitutes for it.

| method | semantics |
|---|---|
| `intValue` | ✔ 123456 |
| `longValue` | ✔ 1234567890123 |
| `floatValue` | ✔ 1.5f |
| `doubleValue` | ✔ 2.25 |
| `charValue` | ✔ 值正确，记法为 120 |
| `booleanValue` | ✔ false |
| `stringValue` | ✔ LABEL |
| `nullValue` | ✔ null |
| `classValue` | ✔ 值正确，记法为临时赋值 |

### Findings

- 所有常量形态还原（int/long/float/double/boolean/string/null 结构 100%）
- `char` 印成数值 120（`'x'` 形态丢失）
- `Class` 字面量变成 `ID ID = ID . class` 临时赋值序列

### Ranked fixes

char 常量保留 `'x'`；类型字面量直接 `X.class`

## Rounds

- R1 2026-11-08: structure 96%, naming 96%; 所有常量形态还原（int/long/float/double/boolean/string/null 结构 100%）
