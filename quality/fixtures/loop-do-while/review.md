# Fixture: loop-do-while

do-while, including a do-while with break. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug loop-do-while`:
  structure 54%, naming 34%.

## Current verdict — round 1, 2026-11-08

### Per method

| method | structure | naming | first divergence |
|---|---|---|---|
| `countUp` | 55% | 33% | `insert: ref=- ours=; ID` |
| `stopEarly` | 55% | 50% | `insert: ref=- ours=; ID` |
| `nested` | 53% | 20% | `delete: ref== 0 ours=-` |

### Findings

- 三个方法都退化成 `while (test { graph<Unstructured> entries [L1] … yield … }) { }`，`do` 完全丢失
- `countUp` 的语义仍正确（`v1 = 0; while (v1 < p0) v1++;`），但形状是语料里最差的
- 结构 53~55%，扣分主要来自图包装与 `;` 插入而不是取值错误

### Ranked fixes

把 `do-while` 识别为循环形态，去掉 `test {}` 与 `graph<Unstructured>`；参数名恢复

## Rounds

- R1 2026-11-08: structure 54%, naming 34%; 三个方法都退化成 `while (test { graph<Unstructured> entries [L1] … yield … }) { }`，`do` 完全丢失
