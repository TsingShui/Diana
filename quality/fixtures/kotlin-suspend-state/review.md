# Fixture: kotlin-suspend-state

suspend functions and the continuation state machine. Written in Kotlin, so the source is the semantic reference only: this
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
| `load` | ✔ 负数抛 IllegalStateException，否则返回 2×value |
| `fetch` | ~ 状态机结构（label 切换、COROUTINE_SUSPENDED、`I$0` 保存）都在，未逐行核对取值 |
| `guarded` | ✔ try/catch 与 `throwOnFailure` 顺序正确 |
| `total` | ~ 循环内挂起，未逐行核对 |

### Findings

- `suspend` 修饰符完全丢失：方法签名是 `Object fetch(int, Continuation<? super Integer>)`
- 状态机以 `switch (label) { case 0: … case 1: … }` + `goto L6/L7` 呈现，`IntrinsicsKt.getCOROUTINE_SUSPENDED()` 与 `ResultKt.throwOnFailure` 裸露
- 出现内部类型关键字 `reference v29;`（不是 Java 也不是 Kotlin）
- `I$0`/`I$1`/`label` 这些 Continuation 字段直接作为成员读写

### Ranked fixes

suspend 形态与状态机折叠；`reference` 关键字出正文；Continuation 字段不暴露

## Rounds

- R1 2026-11-08: not scored (Kotlin source); `suspend` 修饰符完全丢失：方法签名是 `Object fetch(int, Continuation<? super Integer>)`
