# Fixture: terminal-follow-conjunction

## Current verdict — ordered conjunction repair

The pre-fix output for the discovery shape emitted four nested `If` regions and
no `ConditionPlan`: each failed predicate jumped to the common `return false`
block, which was also the branch follow. The candidate owns that abrupt follow as
the shared result arm and composes the field-read/call preludes into the ordered
test operands.

`matches` now evaluates `instanceof`, then the left, middle and right field
equalities in order. A failure reaches `return false`; all four successful tests
reach `return true`. The P4 proof removes the redundant casts after the
successful `instanceof`, so the text uses `p0.left` and similar reads. This is a
readability choice over the original checked local, not a new cast operation.

`effectful` now evaluates `instanceof`, `probe(1)`, the left field equality and
`probe(2)` in that order. Failure reaches `mark(20)` and `return false`; success
reaches `mark(10)` and `return true`. The later probe is not evaluated after an
earlier false test, and no success effect appears on the failure path.

## Evidence boundary

The fixture's independent expectations are in [README](README.md). The focused
engine checks in `short_circuit_routes.rs` inspect the ordinary Engine products:
one four-test `ConditionPlan`, one compact condition layout, the ordered field and
probe text, the two result effects, and one rendered anchor for every original
edge. These are structural and text observations, not a behavioral executor.

The discovery input was the private local `okhttp3.Route.equals` shape. It is not
included in the repository. The same candidate renders it as one condition with
the original four tests and the `(p0 via okhttp3.Route)` check-cast text. This
review does not claim a public release or a device execution.

## Rounds

- R1 2026-11-09: Added the synthetic field-comparison and effectful challenge;
  checked the terminal-follow condition, ordered field/probe evaluation, both
  result arms and original edge anchors. The pre-fix source had no
  `ConditionPlan` for this shape.
