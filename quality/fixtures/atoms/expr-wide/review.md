# Fixture: expr-wide

## Current verdict — selected-route recovery, 2026-11-09

The current LLM review compares the same prepared DEX on committed `29f1b82`
and the uncommitted selected-route candidate. Only `nanComparison` has changed
text. All other method texts are identical, so their prior judgments below are
reused. No behavioral executor, device run or Java recompilation ran.
The same changed paths were reread after integration with main `7354242` in
dev: comparison operators/order, third-branch polarity, original copies and
explicit routing agree, and P1/P3/P4/P5 products remain exact against that main.

### Semantics

| method | semantics |
|---|---|
| `nanComparison` | ~ The original three comparisons and their short-circuit result paths remain explicit and equivalent under the reasoning below. |
| `shifts`, `arithmetic`, `floating`, `single`, `narrowing`, `roundTrip` | ~ Unchanged input and exact unchanged output; the prior method judgments below are reused. |

For NaN, `CmplDouble(value, value)` produces -1 and the first nonzero branch
selects true, skipping the other comparisons. For a non-NaN negative value,
the second `CmpgDouble(value, 0)` result is negative and selects true. Otherwise
the third `CmplDouble(value, 1)` selects true only above 1. Values in [0, 1],
including both signed zeros, select false. Negative and positive infinity take
the negative and above-1 paths respectively. The candidate inverts only the
third branch spelling: its `> 0` edge reaches the retained empty goto and true
result, while its other edge sets false. The baseline `<= 0` branch reached
that same false result. Comparison order, floating operators and 1/0 result
copies are unchanged.

P7 keeps its complete explicit condition alternative because the floating
comparison preludes are not admitted as compact operands. This method therefore
checks routing preservation on the noncompact path; it is not a claim that NaN
comparisons were converted to ordinary Java relations. The extra explicit label
is a readability limitation, not a different destination.


wide and floating-point arithmetic, comparisons and casts. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/ExprWide;`

## Prior verdict — round 1, 2026-11-08

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
- R2 2026-11-09: Reread the changed third-branch spelling and retained routed explicit alternative; other method texts remain exact.
