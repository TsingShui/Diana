# Fixture: expr-wide

wide and floating-point arithmetic, comparisons and casts. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/ExprWide;`

## Current verdict — round 1, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `shifts` | ✔ |
| `arithmetic` | ✔ |
| `floating` | ✔ Mixed int/double promotion agrees. |
| `single` | ✔ The 1.5f/0.25f constants and multiplication agree. |
| `nanComparison` | ✔ NaN behavior and the cmpl/cmpg choice agree. |
| `narrowing` | ✔ Byte and short narrowing after the int conversion agree. |
| `roundTrip` | ✔ Math.round and conversion to long agree. |

### Findings

- nanComparison exposes `compare<CmplDouble>(p0, p0)` / `compare<CmpgDouble>(...)`; the comparison choice agrees, but the notation is decompiler-specific. Recorded forms: `compare<CmpgDouble>(…)`.
- narrowing retains the intermediate int conversion in `((byte) ((int) p0))`, preserving the DEX fact with extra syntax.
- Each subexpression is first stored in a temporary, obscuring the expression structure. Recorded forms: `double v3 = …; double v4 = …`.

### Ranked fixes

Improve cmpl/cmpg and expression presentation while preserving NaN behavior and conversions. Recorded forms: `Double.compare`, `<`, `>`.

## Rounds

- R1 2026-11-08: nanComparison exposes `compare<CmplDouble>(p0, p0)` / `compare<CmpgDouble>(...)`; the comparison choice agrees, but the notation is decompiler-specific. Recorded forms: `compare<CmpgDouble>(…)`.
