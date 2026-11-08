# Fixture: kotlin-range-loops

Kotlin loop forms: `until`, `downTo`, `step`, `repeat`, `forEach` and labelled
`break@`/`continue@`. Written in Kotlin, so the source is the semantic reference
only: this fixture is never scored and the review is the record.

- Provenance: authored for this repository. No third-party code or data.
- Prepared with `kotlinc -jvm-target 1.8` + `d8 --min-api 24 --lib kotlin-stdlib.jar`.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/Case;`

## Current verdict — round 1, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `until` | ~ 未逐行核对 |
| `downTo` | ✔ 变成 `while (v6 > 0) { … v6 = v6 - 1 }`，等价 |
| `stepped` | ~ 未逐行核对（`step` 走 `ProgressionUtilKt`） |
| `repeated` | ~ 未逐行核对 |
| `each` | ✔ `List.forEach` 变迭代器，取值正确 |
| `labelled` | ~ 未逐行核对 `break@outer`/`continue@outer` |

### Findings

- 区间循环全部退化成整数 `while`，`until`/`downTo`/`step`/`..` 语义只能从取值反推。
- `repeat(n) { }` 与 `forEach { }` 变成内联调用 + 迭代器/计数循环。
- 类上出现 `@SourceDebugExtension(value = "SMAP\nCase.kt\nKotlin\n*S Kotlin…")`：整份 SMAP 进正文。
- `@Metadata(d1 = …, d2 = …) runtime` 同样整段进正文；`break@outer` 要靠 goto 落点确认。

### Ranked fixes

区间循环与 `repeat`/`forEach` 的形态还原；SMAP 与 `@Metadata` 移出正文。

## Rounds

- R1 2026-11-08: not scored (Kotlin source); ranges lower to plain integer loops and SMAP enters the class body.
