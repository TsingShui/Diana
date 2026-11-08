# Fixture: loop-map-views

map views and a custom Iterable behind an enhanced for. The Java source is the reference; the DEX is generated from it
with the fixture's `javac.args`/`d8.args` or the shared defaults (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/LoopMapViews;`
- Current numbers from `python3 quality/score.py --slug loop-map-views`:
  structure 64%.

## Current verdict — round 1, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `put` | ✔ |
| `sumValues` | ✔ counts.values() 迭代求和正确 |
| `keys` | ✔ keySet 迭代器正确 |
| `walk` | ✔ 自定义 Iterable 的增强 for 正确 |
| `drainValues` | ~ 未逐行核对 iterator.remove() |

### Score

| method | structure |
|---|---|
| `put` | 71% |
| `sumValues` | 53% |
| `keys` | 82% |
| `walk` | 48% |
| `drainValues` | 68% |

### Findings

- for (int value : counts.values()) 变成 Iterator + Object + ((Integer) v5).intValue()
- 自定义 Iterable 的增强 for 同样变迭代器；Iterable 类型在 cast 里出现
- for (Iterator<Integer> it = …; it.hasNext(); ) 的 for 形态丢失

### Ranked fixes

恢复增强 for 与 Map 视图迭代；元素不经 Object 中转

## Rounds

- R1 2026-11-08: structure 64%; for (int value : counts.values()) 变成 Iterator + Object + ((Integer) v5).intValue()
