# Fixture: obf-flow-r8

R8-obfuscated enum state machine (switch, loop, try/finally, ternary). Built from the same Java reference as its source fixture with an
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
| `advance` | ✔ `when` 各臂与源码等价（START 三元、RUNNING 嵌套 if、else 空臂） |
| `run` | ✔ 循环、`continue`/`break` 与 finally 写回一致（与 complex-flow.run 同体） |
| `summarize` | ✔ 三元展开为临时 + if，等价 |

### Score (structure only: naming is destroyed by the obfuscator)

| method | structure |
|---|---|
| `advance` | 47% |
| `run` | 49% |
| `summarize` | 34% |

### Findings

- R8 重命名后 P7 选择了 Kotlin `when` 决策形态而不是 `switch`：`when (v3) { 0 -> {…} 1 -> {…} else -> { } }`
- `@MemberClasses(value = {class a}) system` 保留；字段名与类型名同字（`private a a;`）
- 结构分比未混淆版本高（advance 47% vs 18%，run 49% vs 21%）：`when` 形态没有 `default:` 提前和 goto，正好更接近源码

### Ranked fixes

确认 `when` 在 Java 输入上是否期望输出；若要 Java 形态，需在 P7 选择 switch 而不是 decision

## Rounds

- R1 2026-11-08: structure 43% (no naming score); R8 重命名后 P7 选择了 Kotlin `when` 决策形态而不是 `switch`：`when (v3) { 0 -> {…} 1 -> {…} else -> { } }`
