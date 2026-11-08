# Fixture: algo-string

KMP prefix/search, word frequency over HashMap and string reversal. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug algo-string`:
  structure 38%, naming 62%.

## Current verdict — round 1, 2026-11-08

### Semantics

The verdict is written by reading the reference next to the output. `score.py`
never substitutes for it.

| method | semantics |
|---|---|
| `prefix` | ✔ 前缀表递推正确 |
| `indexOf` | ✔ 失配回退与命中位置正确 |
| `topWords` | ✔ 计数与过滤正确 |
| `reverse` | ✔ 逆序拼接正确，但 `new StringBuilder` 被拆成 new + `.<init>(len)` |

### Score

| method | structure | naming |
|---|---|---|
| `prefix` | 52% | 57% |
| `indexOf` | 22% | 55% |
| `topWords` | 13% | 67% |
| `reverse` | 63% | 70% |

### Findings

- `topWords` 结构 13%：`Map.Entry` 经 `Object` 取 `getValue`/`getKey` 再 cast，`via Map$Entry` 记号出现
- `reverse` 的构造器调用印成 `new StringBuilder;` + `stringBuilder.<init>(v2)`
- `HashMap`/`ArrayList` 全部原始类型，`Iterator v15 = hashMap.entrySet().iterator()`

### Ranked fixes

构造器合并成 `new X(...)`；保留泛型实参；去掉 `via` 与 `Object` 中转

## Rounds

- R1 2026-11-08: structure 38%, naming 62%; `topWords` 结构 13%：`Map.Entry` 经 `Object` 取 `getValue`/`getKey` 再 cast，`via Map$Entry` 记号出现
