# Fixture: kotlin-sealed-when

sealed hierarchy with exhaustive when plus smart casts. Written in Kotlin, so the source is the semantic reference only: this
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
| `Case` | ✔ `Case(DefaultConstructorMarker)` 合成构造器保留 |
| `describe` | ✔ instanceof 链 + `NoWhenBranchMatchedException`，穷尽语义正确 |
| `labelled` | ✔ 三个分支与字符串模板都正确 |

### Findings

- `when (this) { is Ready -> … }` 变成 `if (!(x instanceof Ready))` 的嵌套否定链，外加 `nop;` 前导
- 子类 `Ready/Failed/Pending` 是独立类，`@MemberClasses` 保留了关系
- `NoWhenBranchMatchedException` 出现在正文，说明穷尽性信息在，只是没用于形态还原

### Ranked fixes

把 instanceof 链还原成 `when`；`test {}`/`nop;` 出正文

## Rounds

- R1 2026-11-08: not scored (Kotlin source); `when (this) { is Ready -> … }` 变成 `if (!(x instanceof Ready))` 的嵌套否定链，外加 `nop;` 前导
