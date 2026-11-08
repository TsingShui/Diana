# Fixture: complex-map-nested

nested generics, computeIfAbsent with a lambda, getOrDefault and entry loops. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug complex-map-nested`:
  structure 54%, naming 69%.

## Current verdict — round 1, 2026-11-08

### Semantics

The verdict is written by reading the reference next to the output. `score.py`
never substitutes for it.

| method | semantics |
|---|---|
| `add` | ✔ `computeIfAbsent` + lambda 类正确 |
| `total` | ✔ null 检查与求和正确 |
| `totals` | ✔ 双重迭代与写回正确 |
| `countOrZero` | ✔ `getOrDefault(key, new ArrayList<>())` |
| `merge` | ✔ `computeIfAbsent` + `addAll` 正确 |

### Score

| method | structure | naming |
|---|---|---|
| `add` | 61% | 50% |
| `total` | 56% | 70% |
| `totals` | 44% | 75% |
| `countOrZero` | 63% | 86% |
| `merge` | 48% | 65% |

### Findings

- `Map<String, List<Integer>>` 声明只留下 `Map groups`（泛型实参全丢），元素取出后再 cast
- `via Map$Entry` 与悬空语句 `((String) key); // Diana: check-cast, value not reassigned` 进入正文
- `entrySet()` 的增强 for 变成 `Iterator` + `Object`

### Ranked fixes

保留嵌套泛型实参；去掉 `via`/悬空 cast；恢复 `for (Map.Entry<…> e : …)`

## Rounds

- R1 2026-11-08: structure 54%, naming 69%; `Map<String, List<Integer>>` 声明只留下 `Map groups`（泛型实参全丢），元素取出后再 cast
