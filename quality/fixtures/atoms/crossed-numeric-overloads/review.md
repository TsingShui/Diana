# Semantic review

The original constructors and instance calls reuse the same received
value for two arguments. The original method references independently
select a char, byte, or int view for each position. The selected
overloads record distinct labels and the integer representation of
both values. The original raw witness verifies the recorder call
and its argument flow from each DEX callee body.

The 0.4.6 publication uses the narrow Java type at both positions
in all six tested crossed calls. Java binds the constructors and
char calls to `(CC)V` and the byte calls to `(BB)V`. All three
behavioral traces record these different overloads while retaining the
observed numeric values and effect order. Java call attribution
independently resolves the same wrong descriptors.

The original same-type controls intentionally pass two char, byte,
or int views. The reconstructed controls preserve all five binding
sites and the recorder's exact values, including int 65536. The
observed error is confined to the recovered overload binding for the
crossed signatures in this fixture.

Diana 0.4.7 applies an integer cast to the second argument of `(CI)`
and `(BI)`, and to the first argument of `(IC)` and `(IB)`. The cast
preserves 65535 for U+FFFF and the sign of negative bytes. Both
constructors and all virtual calls reach the original recorder labels
with the same values and order. The same-type calls preserve their
unconverted formals. The independent attribution and execution checks
observe all these outcomes.
