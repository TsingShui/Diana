# Semantic review

The null condition is evaluated before any predicate on each iteration.
For a non-null input, the left predicate is invoked once. Its success
skips the right predicate and the advance. Left failure reaches the right
predicate exactly once. Right success returns true. Two failures advance
once and recheck the updated input. Advancing to null returns false. A
throw from either call escapes immediately and preserves preceding
observations. In particular, throwing from the first call does not invoke
the second call.

The recovered short-circuit OR has exactly these routes. Its left operand is
the left call, and its right operand is reached only if the left returns
false. The order and protection scope of both calls remain in the original
loop body. The advance remains outside the accepting guard. The model checks
the listed helper behavior and results. The Java statement structure check
independently distinguishes the combined recovery from two disjoint guards.
