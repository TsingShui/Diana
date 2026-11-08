# Fixture: loop-do-while

do-while, including a do-while with break. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug loop-do-while`:
  structure 54%, naming 34%.

## Current verdict — round 2, 2026-11-08

### Per method

| method | structure | naming | first divergence |
|---|---|---|---|
| `countUp` | 55% | 33% | `insert: ref=- ours=; ID` |
| `stopEarly` | 55% | 50% | `insert: ref=- ours=; ID` |
| `nested` | 53% | 20% | `delete: ref== 0 ours=-` |

### Semantics

The semantics verdict is written by reading the reference next to the output; `?` means not yet judged. `score.py` never substitutes for it.

| method | semantics |
|---|---|
| `countUp` | ✔ `do { v1 = v1 + 1; } while (v1 < p0);` |
| `stopEarly` | ✔ `do { … } while (v1 <= p0);` 与原 `if (index > limit) break` 等价 |
| `nested` | ✔ 内外两层都是 `do-while`，取值正确 |

### Findings

- 三个方法现在都是 `do { … } while (cond)`，`graph<Unstructured>` 与空 `[repeat: ]` 记法消失
- 结构分未变（54%）：差距来自 `v1 = (v1 + 1)` 对 `index++`、`LoopDoWhile.state` 全限定名、`nop;`
- `nested` 内层循环后仍有一条 `nop;`（空 block 打印）

### Ranked fixes

`++`/`+=` 复合赋值恢复（影响全部夹具的 token 归一化），空 block 不再打印 `nop;`，再谈参数名

## Rounds

- R1 2026-11-08: structure 54%, naming 34%; 三个方法都退化成 `while (test { graph<Unstructured> entries [L1] … yield … }) { }`，`do` 完全丢失
- R2 2026-11-08: structure 54%, naming 34%; 三方法恢复 `do-while`，内部记法消失；差距转为 `v1 = (v1 + 1)` 与 `nop;`
