# Fixture: kotlin-data-class

data class and the members kotlinc generates (componentN, copy, copy$default, equals/hashCode/toString). Written in Kotlin, so the source is the semantic reference only: this
fixture is **never scored** (`score.py` refuses it) and the review below is the
record. Kotlin has no lexical counterpart in Java-like output, so a number would
measure syntax rather than recovery.

- Provenance: authored for this repository. No third-party code or data.
- Prepared with `kotlinc -jvm-target 1.8` + `d8 --min-api 24 --lib kotlin-stdlib.jar`
  (see `quality/build.sh` output for the versions in use).
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/Case;`

## Current verdict — round 1, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `Case` | ? 未判定 |
| `default` | ? 未判定 |
| `component1` | ~ 未逐行核对 |
| `component2` | ~ 未逐行核对 |
| `copy` | ? 未判定 |
| `describe` | ✔ 语义正确；`':'` 被印成 `append<(char)>(58)` |
| `equals` | ~ 未逐行核对 |
| `first` | ✔ 还原成 `component1()` |
| `getCount` | ✔ |
| `getName` | ✔ |
| `hashCode` | ~ 未逐行核对 |
| `increase` | ✔ 语义正确；暴露成 `copy$default(this, null, v2, 1, null)` |
| `toString` | ~ 未逐行核对 |

### Findings

- `@Metadata(d1 = …, d2 = …, …) runtime` 整段进正文，类声明的 Kotlin 身份只靠它
- `copy$default(Case, String, int, int, Object)` 直接暴露位掩码参数：读起来像 DEX 而不是 Kotlin
- `append<(char)>(58)`：char 字面量退化成数值（与 `complex-annotation` 的枚举值重复是同一类渲染缺陷）
- 属性 getter 保留成 `getName()/getCount()`，`val` 声明形态没有恢复

### Ranked fixes

char 字面量、`copy$default` 与 `@Metadata` 的呈现；属性 → 字段声明的还原

## Rounds

- R1 2026-11-08: not scored (Kotlin source); `@Metadata(d1 = …, d2 = …, …) runtime` 整段进正文，类声明的 Kotlin 身份只靠它
