# Fixture: expr-wide

wide and floating-point arithmetic, comparisons and casts. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/ExprWide;`
- Current numbers from `python3 quality/score.py --slug expr-wide`:
  structure 60%.

## Current verdict — round 1, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `shifts` | ✔ |
| `arithmetic` | ✔ |
| `floating` | ✔ 混合 int/double 的隐式提升正确 |
| `single` | ✔ 1.5f/0.25f 常量与乘法正确 |
| `nanComparison` | ✔ NaN 语义正确（cmpl/cmpg 的选择与源码一致） |
| `narrowing` | ✔ `(byte)((int) p0)`、`(short)((int)(p0>>8))` 正确 |
| `roundTrip` | ✔ `Math.round` + `(long)` 正确 |

### Score

| method | structure |
|---|---|
| `shifts` | 71% |
| `arithmetic` | 62% |
| `floating` | 48% |
| `single` | 72% |
| `nanComparison` | 27% |
| `narrowing` | 77% |
| `roundTrip` | 65% |

### Findings

- `nanComparison` 出现非 Java 记法 `compare<CmplDouble>(p0, p0)` / `compare<CmpgDouble>(…)`：cmpl/cmpg 的选择本身正确，但记号不是 Java
- `narrowing` 保留了中间 `(int)` 转换：`((byte) ((int) p0))`，是 DEX 事实但比源码多一层
- 结构 27%~77%，扣分来自每个表达式先存临时（`double v3 = …; double v4 = …`）

### Ranked fixes

cmpl/cmpg 用 `Double.compare`/`<`/`>` 的等价形式表达；表达式不再逐步存临时

## Rounds

- R1 2026-11-08: structure 60%; `nanComparison` 出现非 Java 记法 `compare<CmplDouble>(p0, p0)` / `compare<CmpgDouble>(…)`：cmpl/cmpg 的选择本身正确，但记号不是 Java
