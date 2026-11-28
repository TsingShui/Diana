# JVM signed zero selection review

The original float selector uses `ldc` for negative zero and `fconst_0`
for positive zero. The double selector uses `ldc2_w` for negative zero
and `dconst_0` for positive zero. Each joined value is still distinguishable
through the raw-bit observations despite both zeros being numerically equal.

Frozen 0.4.20 collapses the two pure operand alternatives. On true, the
float argument reports `00000000` instead of `80000000`. The double argument
reports `8000000000000000` instead of `0000000000000000`. The static fields
observe the same sign changes. In the nested selection all three paths that
originally produced negative double zero become positive. Only the original
positive path agrees.

`guardedStore` joins the alternatives before `fastore`. Its true path in the
baseline writes positive zero, returning `00000000` and keeping trace 12 and
the other array cell. Both bad-index and null paths retain their original
handler outcomes. The effectful selector evaluates its helper once. Direct
returns, same-sign arguments and the four `ConstantValue` declarations
retain their original signs on the baseline.

The repair preserves the `-0.0` and `0.0` alternatives in both call
arguments, both static writes and the protected array store. The nested
double selection emits one condition, `outer && !inner`, and retains both
signed values. Execution of the exact recompiled class matches all 25
independently expected results, effects and fault traces.
