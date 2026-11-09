# Fixture: expr-wide

## Current verdict — whole-operation follow-up

Compared the same prepared DEX with completed-eleven-repair bda9a49. Only
derived label/control coordinates change. Development-only correspondence found
identical printed words, original L/H program points and identical printed
lexical-scope bindings across every method publication; declaration-only members
are exact. Original P2/P3/P4/P5 facts agree. The unchanged semantic path review
below is reused, not replaced by a naming score or a claim of behavioral
execution.

The [pure-tail challenge](../../exceptions/pure-tail-finally/review.md) and
[carrier challenge](../../atoms/refined-carriers/review.md) record the owning
producer/consumer repairs and blocked drafts. This is bounded current source
reading; historical execution claims below were not rerun and do not certify
this output. No behavioral executor, device run or Java recompilation ran.


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
