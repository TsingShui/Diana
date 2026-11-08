# Fixture: complex-indy-concat

string concatenation through StringConcatFactory (invoke-custom). The Java source is the reference; the DEX is generated from it
with the fixture's `javac.args`/`d8.args` or the shared defaults (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/ComplexIndyConcat;`
- Current numbers from `python3 quality/score.py --slug complex-indy-concat`:
  structure 28%.

## Current verdict — round 1, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `simple` | ✔ call_site { … makeConcat … } + invoke<Custom>，bootstrap 与参数完整 |
| `many` | ~ 未逐行核对 6 个参数的类型提升 |
| `insideLoop` | ~ 未逐行核对循环内的 indy 调用 |

### Score

| method | structure |
|---|---|
| `simple` | 28% |
| `many` | 25% |
| `insideLoop` | 31% |

### Findings

- invoke-custom 用显式形态打印：call_site r2 CallSite(CallSiteId(2)) { name: makeConcat; … bootstrap: StringConcatFactory.makeConcat<…>; } + invoke<Custom> call_site<r2; …>(p0, ":", p1)
- 结构分只有 25%~32%：一行字符串拼接变成两行元数据 + 一次调用
- 所有 indy 事实（bootstrap 方法、proto、参数个数）都保留，没有编造

### Ranked fixes

给 StringConcatFactory 一个可读形态（a + ":" + b），call_site 元数据移到注释/JSON

## Rounds

- R1 2026-11-08: structure 28%; invoke-custom 用显式形态打印：call_site r2 CallSite(CallSiteId(2)) { name: makeConcat; … bootstrap: StringConcatFactory.makeConcat<…>; } + invoke<Custom> call_site<r2; …>(p0, ":", p1)
