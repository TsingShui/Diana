# Fixture: kotlin-object-singleton

object singleton and companion object. Written in Kotlin, so the source is the semantic reference only: this
fixture is **never scored** (`score.py` refuses it) and the review below is the
record. Kotlin has no lexical counterpart in Java-like output, so a number would
measure syntax rather than recovery.

- Provenance: authored for this repository. No third-party code or data.
- Prepared with `kotlinc -jvm-target 1.8` + `d8 --min-api 24 --lib kotlin-stdlib.jar`
  (see `quality/build.sh` output for the versions in use).
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/Counter;`

## Current verdict — round 1, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `Counter` | ✔ `INSTANCE` + 私有构造器 + static 块，正是 Kotlin object 的编译形态 |
| `add` | ✔ 静态 `total` 累加正确 |
| `getTotal` | ✔ |

### Findings

- object 的 `INSTANCE` 形态被完整保留：`public static final Counter INSTANCE` + static 块 `new Counter()`
- companion 出现我们的内部标记 `static_preparation_before_clinit { Companion = null; PREFIX = "case:"; created = implicit_default; }`
- 合成访问器 `access$getCreated$cp` / `access$setCreated$cp` 作为普通方法出现在正文

### Ranked fixes

`static_preparation_before_clinit` 与 `implicit_default` 移出正文；companion 的常量直接内联

## Rounds

- R1 2026-11-08: not scored (Kotlin source); object 的 `INSTANCE` 形态被完整保留：`public static final Counter INSTANCE` + static 块 `new Counter()`
