# Fixture: complex-inner

anonymous class, member inner, static nested and a generic bag. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug complex-inner`:
  structure 74%, naming 36%.

## Current verdict — round 1, 2026-11-08

### Semantics

The verdict is written by reading the reference next to the output. `score.py`
never substitutes for it.

| method | semantics |
|---|---|
| `adding` | ✔ 捕获以构造参数出现（`new ComplexInner$1(this, p0)`） |
| `runVisitor` | ✔ 两次 visit 顺序正确 |
| `runAccumulator` | ✔ 累加正确 |
| `runBag` | ✔ 泛型实参丢失（`new ComplexInner$Bag()`），取值正确 |

### Score

| method | structure | naming |
|---|---|---|
| `adding` | 52% | 14% |
| `runVisitor` | 97% | 43% |
| `runAccumulator` | 63% | 38% |
| `runBag` | 82% | 50% |

### Findings

- 匿名类的捕获通过 `ComplexInner$1(this, p0)` 表达，语义正确但类名是合成名
- 泛型类型实参全部丢失：`Bag<String> bag = new Bag<>()` 印成 `ComplexInner$Bag bag = new ComplexInner$Bag()`
- `runVisitor` 结构 97%，其余方法主要差在临时变量与循环形态

### Ranked fixes

恢复泛型实参；匿名类可读性（捕获字段与 `run` 方法）单独评审

## Rounds

- R1 2026-11-08: structure 74%, naming 36%; 匿名类的捕获通过 `ComplexInner$1(this, p0)` 表达，语义正确但类名是合成名
