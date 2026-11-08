# Fixture: complex-iteration

iterator loops, enhanced for, map entries and removal via iterator. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug complex-iteration`:
  structure 67%, naming 59%%.

## Current verdict — round 1, 2026-11-08

### Semantics

The verdict is written by reading the reference next to the output. `score.py`
never substitutes for it.

| method | semantics |
|---|---|
| `add` | ✔ |
| `totalWeight` | ✔ entrySet 遍历求和 |
| `dropShort` | ✔ `iterator.remove()` 保留 |
| `heavy` | ? 未读 |
| `sumNested` | ✔ 二维增强 for 展开为下标循环，取值正确 |

### Score

| method | structure | naming |
|---|---|---|
| `add` | 75% | 71% |
| `totalWeight` | 49% | 50% |
| `dropShort` | 92% | 82% |
| `heavy` | 66% | 71% |
| `sumNested` | 49% | 20% |

### Findings

- `for (Map.Entry<…> entry : counts.entrySet())` 变成 `Iterator` + `Object` + `((Map$Entry) v5).getValue()`
- `for (String name : names)` 变成 `Iterator` + `((String) v5).length()`
- `heavy` 的 `counts.get(name)` 与空值判断未核实

### Ranked fixes

增强 for 与 Map.Entry 形态恢复；保留泛型实参；元素不再经 `Object` 中转

## Rounds

- R1 2026-11-08: structure 66%, naming 59%; `for (Map.Entry<…> entry : counts.entrySet())` 变成 `Iterator` + `Object` + `((Map$Entry) v5).getValue()`
