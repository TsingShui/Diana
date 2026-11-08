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

## Current verdict — round 2

当前普通 Engine 输出已恢复循环内资源清理的保护层级。下表分数保留 R1；本轮未重跑 score。

### Semantics

| method | semantics |
|---|---|
| `parseOrThrow` | ✔ 构造器与 cause 正确（`new X;` + `x.<init>` 的记法问题依旧） |
| `nestedFinally` | ✔ 内层/外层 finally 两条路径都执行 |
| `resources` | ? 混淆后的资源循环同样恢复嵌套 try/catch；独立行为场景尚未执行 |
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
- TWR-in-loop 的读取及私有清理 handler 已同属循环体，关闭失败的 suppression 保留在内层 catch；恢复不依赖原类名或成员名

### Ranked fixes

执行关闭与 suppressed 顺序的独立场景；输入已丢失的嵌套类属性不能凭名字补造

## Rounds

- R2: 混淆版本的资源循环与未混淆版本同时恢复保护层级；行为验证保留 ?，未重测结构分。

- R1 2026-11-08: structure 31% (no naming score); ProGuard 把 `ComplexExceptions` 命名为 `a.a`、`Res` 命名为 `a.b`（**嵌套类拍平**），`@MemberClasses` 丢失
