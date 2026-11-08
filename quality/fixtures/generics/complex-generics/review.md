# Fixture: complex-generics

generic class, generic method with a bound, bridge method and generic call. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug complex-generics`:
  structure 72%, naming 58%.

## Current verdict — round 1, 2026-11-08

### Semantics

The verdict is written by reading the reference next to the output. `score.py`
never substitutes for it.

| method | semantics |
|---|---|
| `largest` | ✔ 擦除后的 `Comparable` 比较正确；`<T extends Comparable<T>>` 只留在 `@Signature` |
| `repeat` | ✔ 泛型 `List<T>` 恢复 |
| `applyMapper` | ✔ `Mapper<String>` 与 cast 正确 |

### Score

| method | structure | naming |
|---|---|---|
| `largest` | 56% | 75% |
| `repeat` | 81% | 50% |
| `applyMapper` | 79% | 50% |

### Findings

- 两个方法对泛型的处理不一致：`repeat` 恢复出 `<T> List<T>`，`largest` 只能给出 `java.lang.Comparable largest(java.util.List)` 加一条 `@Signature(...) system`
- `@Signature`/`@MethodParameters` 以注解文本进入正文并带 `system` 尾词
- `Identity.apply` 的桥接方法未被单独评审（内部类单独渲染）

### Ranked fixes

统一从 `@Signature` 恢复泛型签名；注解不进正文；参数名恢复

## Rounds

- R1 2026-11-08: structure 72%, naming 58%; 两个方法对泛型的处理不一致：`repeat` 恢复出 `<T> List<T>`，`largest` 只能给出 `java.lang.Comparable largest(java.util.List)` 加一条 `@Signature(...) system`
