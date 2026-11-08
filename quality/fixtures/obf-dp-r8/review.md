# Fixture: obf-dp-r8

R8-obfuscated edit distance, LIS and knapsack. Built from the same Java reference as its source fixture with an
obfuscator, so the decompiled bodies can be compared against a known-good
reference.

- Provenance: authored for this repository. No third-party code or data; the
  obfuscated DEX is generated from our own source.
- Toolchain: R8, or ProGuard when the rules file starts with `# obfuscator:
  proguard`; `-dontshrink -dontoptimize` unless the rules say otherwise.
  `quality/build.sh` runs whichever tool the fixture's own rules file names.
- `mapping.txt` records the renaming. `score.py` pairs methods through it,
  matching name and parameter types, so the score stays comparable with the
  source fixture even when the obfuscator reuses one short name for overloads.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — round 1, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `editDistance` | ✔ 与 algo-dp.editDistance 同体（2D 数组仍走 `Array.newInstance`） |
| `longestIncreasing` | ✔ 同体 |
| `knapsack` | ✔ 同体 |

### Score (structure only: naming is destroyed by the obfuscator)

| method | structure |
|---|---|
| `editDistance` | 16% |
| `longestIncreasing` | 9% |
| `knapsack` | 64% |

### Findings

- 方法与未混淆版本逐项同分（16%/9%/64% vs 14%/9%/64%），说明只重命名没有额外结构损伤
- 方法名压成 `a`/`b`/`c`；`Array.newInstance` + 缺 cast 的读取依旧出现
- `mapping.txt` 让 `score.py` 能按名字+参数类型把混淆方法配对回源码方法

### Ranked fixes

同 algo-dp：二维数组声明与访问、循环形态

## Rounds

- R1 2026-11-08: structure 30% (no naming score); 方法与未混淆版本逐项同分（16%/9%/64% vs 14%/9%/64%），说明只重命名没有额外结构损伤
