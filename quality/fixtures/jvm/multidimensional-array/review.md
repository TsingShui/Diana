# Multidimensional array recovery review

The original `Tensor.partial` has `multianewarray` at byte 12 with rank
three and two allocated dimensions. The original `guarded` has the opcode
at byte 23 with three allocated dimensions. OpenJDK `javap` confirmed both
operands. The previous JVM source implementation reported those positions
as unavailable.

The recovered partial path calls the two original helpers in order, with
`(1, rows)` followed by `(2, columns)`. It allocates only the two outer
dimensions and returns the original reference-array rank. The three
helpers in `guarded` run in ordinal order before the allocation. A
negative dimension preserves the typed catch, including the path where
a zero outer length precedes the negative inner length. The catch reads
the trace after all three calls. The normal return reads the outer length.

The original Java and the separately recompiled recovered Java both
passed all four independent `CheckTensor` scenarios. The test does not
exercise allocation failure or custom class loaders.
