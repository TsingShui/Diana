# Fixture: complex-stream

stream pipelines, method references and Optional. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug complex-stream`:
  structure 62%, naming 73%.

## Current verdict — round 1, 2026-11-08

### Semantics

The verdict is written by reading the reference next to the output. `score.py`
never substitutes for it.

| method | semantics |
|---|---|
| `evensDoubled` | ✔ `filter/map/collect` 顺序正确，lambda 体在 `lambda$evensDoubled$0/1` |
| `upper` | ✔ 两个方法引用各成 `$r8$lambda$…` 与 `ComplexStream$0` |
| `sumOver` | ✔ |
| `firstPositive` | ✔ `Optional.orElse(-1)` 正确 |
| `fromArrays` | ✔ `IntStream.concat(…).forEach(list::add)`，`Objects.requireNonNull` 是 d8 的方法引用插桩 |

### Score

| method | structure | naming |
|---|---|---|
| `evensDoubled` | 57% | 82% |
| `upper` | 58% | 73% |
| `sumOver` | 70% | 71% |
| `firstPositive` | 62% | 73% |
| `fromArrays` | 63% | 67% |

### Findings

- lambda 体方法以 `lambda$…$N` 与 `$r8$lambda$…` 的名字出现在类体，且带 `dex_flags<synthetic>`
- 每个 lambda 都先 `new ComplexStream$N()` 再传入 `filter/map/forEach`，中间结果全部落到 `Stream v4/v6` 临时
- `Collector v7 = Collectors.toList(); Object v8 = v6.collect(v7); return ((List) v8);` 三行代替一行管道

### Ranked fixes

lambda 与管道形态恢复（`values.stream().filter(…).map(…).collect(…)`）；合成名与 `dex_flags<>` 出正文

## Rounds

- R1 2026-11-08: structure 62%, naming 73%; lambda 体方法以 `lambda$…$N` 与 `$r8$lambda$…` 的名字出现在类体，且带 `dex_flags<synthetic>`
