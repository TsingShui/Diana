# Fixture: class-inner-static

static nested class with its own state. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Current numbers from `python3 quality/score.py --slug class-inner-static`:
  structure 89%, naming 54%.

## Current verdict — round 1, 2026-11-08

### Per method

| method | structure | naming | first divergence |
|---|---|---|---|
| `holder` | 82% | 33% | `replace: ref=return ours=ID ID =` |
| `useHolder` | 95% | 75% | `insert: ref=- ours=. ID` |

### Findings

- `holder()` 82%：构造后经临时返回；`useHolder()` 95%：仅多一次 `.` 限定
- 静态嵌套类单独渲染，外部引用保留原类名

### Ranked fixes

返回构造结果时不要中转临时；参数名恢复

## Rounds

- R1 2026-11-08: structure 89%, naming 54%; `holder()` 82%：构造后经临时返回；`useHolder()` 95%：仅多一次 `.` 限定
