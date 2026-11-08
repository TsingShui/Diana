# Fixture: class-anonymous

anonymous class capturing a local and a field. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug class-anonymous`:
  structure 73%, naming 67%.

## Current verdict — round 1, 2026-11-08

### Per method

| method | structure | naming | first divergence |
|---|---|---|---|
| `task` | 55% | 33% | `delete: ref=final ours=-` |
| `currentBase` | 91% | 100% | `insert: ref=- ours=this .` |

### Semantics

The semantics verdict is written by reading the reference next to the output; `?` means not yet judged. `score.py` never substitutes for it.

| method | semantics |
|---|---|
| `task` | ✔ 捕获正确（this + step） |
| `currentBase` | ✔ |

### Findings

- `task()` 54.5%：`new ClassAnonymous$1(this, p0)` 结构与捕获正确，但先赋给临时再返回
- `final` 参数修饰被丢弃（对语义无影响，属记法差异）
- 匿名类本体单独渲染，捕获通过构造参数出现

### Ranked fixes

直接 `return new ...(...)`；参数名恢复；匿名类本体的捕获字段与构造参数可读性单独评审

## Rounds

- R1 2026-11-08: structure 73%, naming 67%; `task()` 54.5%：`new ClassAnonymous$1(this, p0)` 结构与捕获正确，但先赋给临时再返回
