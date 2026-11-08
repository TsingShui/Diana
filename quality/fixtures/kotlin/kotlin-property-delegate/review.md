# Fixture: kotlin-property-delegate

delegated properties, accessor pair and lateinit. Written in Kotlin, so the source is the semantic reference only: this
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
| `0` | ? 未判定 |
| `getCached` | ~ 未逐行核对 |
| `cache` | ✔ 走 `getCached()`，委托字段 `cached$delegate` 保留 |
| `getLogged` | ~ 未逐行核对 |
| `getObserved` | ~ 未逐行核对 |
| `getReady` | ? 未判定 |
| `prepare` | ✔ `setReady("ready")` 正确 |
| `readyLength` | ✔ `this.ready == null` 即 `::ready.isInitialized` 的降级，正确 |
| `setLogged` | ~ 未逐行核对 |
| `setObserved` | ~ 未逐行核对 |
| `setReady` | ? 未判定 |

### Findings

- 委托属性展开成 `$delegate` 字段 + `LazyKt.lazy(lambda)` + `ReadWriteProperty` 调用，装饰器语义在但形态全丢
- `$$delegatedProperties` 数组用 `Reflection.mutableProperty1(new MutablePropertyReference1Impl(...))` 构造，且构造器被拆成 `new X;` + `.<init>`
- `@Signature(KProperty<Object>[]) system` 这种注解进正文；`lateinit` 变成普通可空字段 + null 检查

### Ranked fixes

委托属性的 `by lazy`/`by Delegate` 形态；`new X(...)` 合并；注解与内部标记出正文

## Rounds

- R1 2026-11-08: not scored (Kotlin source); 委托属性展开成 `$delegate` 字段 + `LazyKt.lazy(lambda)` + `ReadWriteProperty` 调用，装饰器语义在但形态全丢
