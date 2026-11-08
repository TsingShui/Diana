# Fixture: complex-annotation

runtime annotations with array, enum, class and default values. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/ComplexAnnotation;`
- Current numbers from `python3 quality/score.py --slug complex-annotation`:
  structure 92%.

## Current verdict — round 1, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `read` | ✔ 方法注解保留 |
| `write` | ✔ 只保留显式设置的值（默认值不展开，符合 DEX） |

### Score

| method | structure |
|---|---|
| `read` | 91% |
| `write` | 93% |

### Findings

- 类注解的枚举值渲染错误：`@Marker(color = GREEN ComplexAnnotation$Color ComplexAnnotation$Color, name = …)` —— 值、类型名重复两遍
- 注解尾部带可见性词：`@ComplexAnnotation$Marker(…) runtime`
- `type = class String` 与 `numbers = {1, 2, 3}` 正确；嵌套注解类以独立类渲染

### Ranked fixes

修枚举注解值的重复渲染；可见性词移出正文

## Rounds

- R1 2026-11-08: structure 92%; 类注解的枚举值渲染错误：`@Marker(color = GREEN ComplexAnnotation$Color ComplexAnnotation$Color, name = …)` —— 值、类型名重复两遍
