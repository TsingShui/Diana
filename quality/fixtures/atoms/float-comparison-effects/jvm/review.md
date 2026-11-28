# JVM floating-point compare recovery review

The original `one()Z` invokes `left()` and `right()` at offsets 0 and 3
before `fcmpg` at offset 6. With inputs 2 and 1 the predicate returns true
after trace `12`. The frozen 0.4.15 publication repeats both calls in the
NaN, less-than and equality clauses and still returns true after trace
`121212`. The repair stages the two calls in locals, then tests their
captured results.

In `nanSkips(Ljava/lang/RuntimeException;)Z`, `leftNaN()` records 1 and
returns NaN. `rightMaybe()` then records 2. The null exception argument
allows `fcmpg` to produce +1 and the branch to return true. A non-null
argument causes `rightMaybe()` to throw the supplied object before
`fcmpg` executes. The baseline puts the right call behind the short-circuit
NaN test. It returns true in both cases at trace `1`. The repair calls
`rightMaybe()` before entering that test, so it retains the trace and
propagates the same exception object in the throwing case.

`nestedOperands(FF)Z` negates both operands at offsets 4 and 9 before
`fcmpg`. The repair stages both signed values in source order before the
predicate. `nestedResult(FF)Z` adds 1 to the `fcmpl` result at offset 10
before testing equality against 2. The repair keeps the addition and the
negative NaN bias and stages both producers outside the comparison
expansion. The four ordinal methods use their original bias for unordered
values and treat positive and negative zero as equal. The `zero()Z`
control keeps its compact single-use relation and invokes each operand once.

The exact repaired publication compiled and matched the original for the
75 scenarios in [the independent witness](CheckFloatCompare.java).
