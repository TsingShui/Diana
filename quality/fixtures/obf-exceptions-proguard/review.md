# Fixture: obf-exceptions-proguard

ProGuard-obfuscated nested try/finally, TWR in a loop and multi-catch. Built from the same Java reference as its source fixture with an
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
| `parseOrThrow` | ✔ 构造器与 cause 正确（`new X;` + `x.<init>` 的记法问题依旧） |
| `nestedFinally` | ✔ 内层/外层 finally 两条路径都执行 |
| `resources` | ✘ TWR-in-loop：`H6` 在 `goto L10` 之后、`try { H6: … }` 出现在 `throw` 之后 |
| `multiAndInstance` | ✔ 两个 catch 的返回与 multi 分支正确 |

### Score (structure only: naming is destroyed by the obfuscator)

| method | structure |
|---|---|
| `parseOrThrow` | 43% |
| `nestedFinally` | 17% |
| `resources` | 45% |
| `multiAndInstance` | 21% |

### Findings

- ProGuard 把 `ComplexExceptions` 命名为 `a.a`、`Res` 命名为 `a.b`（**嵌套类拍平**），`@MemberClasses` 丢失
- 同名重载严重：`parseOrThrow`/`nestedFinally`/`multiAndInstance` 全部叫 `a`，靠参数类型区分——`score.py` 必须用 mapping 里的签名配对，否则配错
- TWR-in-loop 的 handler 归属缺陷原样保留（与未混淆版本一致）

### Ranked fixes

先修 handler 归属；若要保留嵌套类信息需在 rules 里加 `-keepattributes InnerClasses`

## Rounds

- R1 2026-11-08: structure 31% (no naming score); ProGuard 把 `ComplexExceptions` 命名为 `a.a`、`Res` 命名为 `a.b`（**嵌套类拍平**），`@MemberClasses` 丢失
