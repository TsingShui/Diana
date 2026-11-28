# Semantic review

The original if-test consumes the comparison's integer result. A NaN
operand yields +1 for the float comparison and -1 for the double
comparison. Either unordered operand therefore sends control directly
to the smaller arm. The additional boolean condition in the gated
case is reached only after the original float branch accepts its ordered
less-than operands.

When the two paths share the return, Diana 0.4.19 moves the smaller arm
into a guard. It complements the relational operator instead of
negating the truth value of the comparison. For float, `!(a < b)` is
true for unordered inputs while `a >= b` is false. The double
`!(a > b)` has the corresponding behavior against `a <= b`. The
wrong guard sends the original unordered inputs to the larger arm.
The execution reaches a different effect sequence even though the
method is void and its operands have no effectful producers.

For finite ordered and equal values, the two guard forms select the
same arm. The compiler's integer branch keeps both extrema and
equality in the original order. The float and double direct-return
controls preserve the original smaller-arm result on NaN in 0.4.19.

The repaired publication keeps `!(f < f2)` and `!(d > d2)` as the
early guards for the joined arms. Their unordered paths reach
`mark(9)`. The gated method still selects the larger arm for strictly
ordered inputs when the gate is true. Ordinary finite,
signed-zero and integer paths retain their original traces. All 16
executed scenarios pass when the exact repaired class is recompiled.
