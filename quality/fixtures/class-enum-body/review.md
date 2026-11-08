# Fixture: class-enum-body

enum with constructor state, a method and a constant body. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug class-enum-body`:
  structure 90%, naming 83%.

## Current verdict — round 1, 2026-11-08

### Per method

| method | structure | naming | first divergence |
|---|---|---|---|
| `weight` | 91% | 100% | `insert: ref=- ours=this .` |
| `scaled` | 89% | 67% | `insert: ref=- ours=( this .` |

### Semantics

The semantics verdict is written by reading the reference next to the output; `?` means not yet judged. `score.py` never substitutes for it.

| method | semantics |
|---|---|
| `weight` | ✔ |
| `scaled` | ✔ |

### Findings

- `weight()` 91%、`scaled()` 89%：方法体正确
- 枚举常量没有还原成 `FIRST(1), SECOND(2) { … }`，而是 `public static final dex_flags<enum> … ` + static 块里的 `new ClassEnumBody("FIRST", 0, 1)`
- 常量体类 `ClassEnumBody$1` 在 static 块里被直接 `new`，方法体重写不可见
- `dex_flags<enum>`/`dex_flags<synthetic>` 与 `@… system` 进入正文

### Ranked fixes

枚举常量与常量体还原成声明形式；`dex_flags<>`/`system` 移出正文；构造器参数名恢复

## Rounds

- R1 2026-11-08: structure 90%, naming 83%; `weight()` 91%、`scaled()` 89%：方法体正确
