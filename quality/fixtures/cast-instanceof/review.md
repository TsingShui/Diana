# Fixture: cast-instanceof

instanceof / check-cast. The Java source is the reference and the expectation; the DEX is
generated from it with `javac --release 8 -g` + `d8 --min-api 24` (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug cast-instanceof`:
  structure 87%, naming 77%.

## Current verdict — round 1, 2026-11-08

### Per method

| method | structure | naming |
|---|---|---|
| `isString` | 94% | 75% |
| `castToNumber` | 90% | 80% |
| `castAfterTest` | 81% | 86% |
| `arrayCast` | 84% | 67% |

### Findings

- `instanceof` 与 `(Number) v1` 转换正确
- `castToNumber` 多出一次 `Object v1 = p0;` 临时；数组 cast `(int[]) value` 正确

### Ranked fixes

cast 直接作用于原值，不再中转临时变量；参数名恢复

## Rounds

- R1 2026-11-08: structure 87%, naming 77%; `instanceof` 与 `(Number) v1` 转换正确
