# Null binding semantic review

The complete declaration for the selected generic helper is in the original
class. The caller's receiver signature establishes the input type that the
generic declaration accepts. Both publication endpoints use those facts.

- The original `object` method passes a null reference to the erased
  `pick(Object)`. Diana 0.4.28 shows an erased cast in full-class output,
  which makes both source overloads inapplicable for the `Number` receiver.
  The selected method shows bare null, which recompiles to `pick(Integer)`
  and produces trace 2. The candidate's `Number` annotation rebinds both
  endpoints to the original helper and preserves trace 1. The intentionally
  narrower `integer` control still binds to `pick(Integer)`.
- The guarded candidate evaluates `leading` before the consumer and retains
  the original typed protection over both calls. Its null argument is a
  `Number[]`, so it binds to the instantiated generic overload. That call
  records 56 on success. The baseline selected method chooses the competing
  array descriptor and records 57. If the leading operand throws, the
  consumer is skipped. The handler observes the supplied exception object
  and records 58.
- The bounded control omits the external signature from the analyzed input.
  Its bare null still leaves the original intersection bound admissible.
  The observer sees the same `bounded/9` trace.

The review covers the authored paths and JVM definitions in this fixture.
External overloads whose required declaration facts are absent can remain
ambiguous in reconstructed source.
