# Fixture: cast-instanceof

instanceof / check-cast. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug cast-instanceof`:
  structure 84%, naming 77%%.

## Current verdict — round 2, 2026-11-08

### Per method

| method | structure | naming |
|---|---|---|
| `isString` | 94% | 75% |
| `castToNumber` | 85% | 80% |
| `castAfterTest` | 81% | 86% |
| `arrayCast` | 79% | 67% |

### Semantics

The semantics verdict is written by reading the reference next to the output; `?` means not yet judged. `score.py` never substitutes for it.

| method | semantics |
|---|---|
| `isString` | ✔ |
| `castToNumber` | ✔ 多一次临时 |
| `castAfterTest` | ✔ 反演但等价 |
| `arrayCast` | ✔ 多一次临时 |

### Findings

- `instanceof` 与 `(Number) v1` 转换正确
- 检查折到声明处：`int[] v1 = (int[]) p0; return v1;`、`Number v1 = (Number) p0;`
- 结构分 87% → 84% 是形态代价：参考里 `return (int[]) value` 是一行，现在拆成声明加返回

### Ranked fixes

cast 直接作用于原值，不再中转临时变量；参数名恢复

## Rounds

- R2 2026-11-08: structure 84%, naming 77%; 转换折进声明，`arrayCast`/`castToNumber` 不再有 `Object` 临时；结构分小幅下降是行数形态变化
- R1 2026-11-08: structure 87%, naming 77%; `instanceof` 与 `(Number) v1` 转换正确
