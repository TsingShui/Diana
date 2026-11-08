# Fixture: class-init-order

field initializers, an instance block, static init and this(...). The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug class-init-order`:
  structure 68%, naming 100%.

## Current verdict — round 1, 2026-11-08

### Per method

| method | structure | naming | first divergence |
|---|---|---|---|
| `total` | 45% | 100% | `replace: ref=return ours=int ID = this .` |
| `label` | 91% | 100% | `insert: ref=- ours=. ID` |

### Semantics

The semantics verdict is written by reading the reference next to the output; `?` means not yet judged. `score.py` never substitutes for it.

| method | semantics |
|---|---|
| `total` | ✔ |
| `label` | ✔ |

### Findings

- 静态块、实例字段初始化、实例块的**顺序正确**（`second = 2` → 实例块 → `first = p0`）
- `total()` 44.8%：每个字段先取到局部再相加（`int first = this.first; …`）
- 无参构造器正确保留 `this(1);`；`static final LABEL` 经临时赋入静态块

### Ranked fixes

字段读取不再中转局部变量；`static final` 直接赋字面量；参数名恢复

## Rounds

- R1 2026-11-08: structure 68%, naming 100%; 静态块、实例字段初始化、实例块的**顺序正确**（`second = 2` → 实例块 → `first = p0`）
