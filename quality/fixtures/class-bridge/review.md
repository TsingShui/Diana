# Fixture: class-bridge

bounded generics, a real bridge method and wildcard signatures. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/ClassBridge;`
- Current numbers from `python3 quality/score.py --slug class-bridge`:
  structure 77%.

## Current verdict — round 1, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `countAbove` | ✔ |
| `copyInto` | ✔ 100%（`List<? super T>` 的 `addAll` 完全一致） |

### Score

| method | structure |
|---|---|
| `countAbove` | 54% |
| `copyInto` | 100% |

### Findings

- 桥接方法被标成 `public dex_flags<bridge synthetic> int compare(Object, Object)`：身份对，记号是内部的
- 泛型上界丢失：`Box<T extends Comparable<T>>` 只剩 `implements java.util.Comparator` + 一条 `@Signature`，而 `@Signature` 文本被切成 `<T::`、`Ljava/lang/Comparable<` 之类片段
- `private final java.util.List items` 原始类型；`copyInto` 却完全正确（100%），说明信息在、只是没统一用

### Ranked fixes

从 `@Signature` 恢复泛型声明与上界；`dex_flags<bridge synthetic>` 移出正文

## Rounds

- R1 2026-11-08: structure 77%; 桥接方法被标成 `public dex_flags<bridge synthetic> int compare(Object, Object)`：身份对，记号是内部的
