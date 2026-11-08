# Fixture: class-interface-default

interface default and static methods. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug class-interface-default`:
  structure 78%, naming 89%.

## Current verdict — round 1, 2026-11-08

### Per method

| method | structure | naming | first divergence |
|---|---|---|---|
| `doubled` | 81% | 100% | `replace: ref=default ours=public` |
| `plus` | 59% | 67% | `replace: ref=default ours=public` |
| `constant` | 95% | 100% | `insert: ref=- ours=public` |

### Findings

- `default` 关键字丢失：输出是 `public int doubled()`（有方法体的接口方法没有声明符）
- `constant()` 95%、`plus()` 条件被反演（`if (p0 >= 0)` 与源码相反但等价）
- 分词器此前漏掉 `default` 方法，本轮已修（语料方法数 86 → 88）

### Ranked fixes

还原 `default` 关键字；条件不反演；参数名恢复

## Rounds

- R1 2026-11-08: structure 78%, naming 89%; `default` 关键字丢失：输出是 `public int doubled()`（有方法体的接口方法没有声明符）
