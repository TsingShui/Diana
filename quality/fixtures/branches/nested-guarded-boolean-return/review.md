# Nested guarded boolean return review

Diana 0.4.30 already groups the first three predicates into a conditional
guard and uses a second guard and constant fallback. Diana 0.4.31 produces
`first() && (!second() || third()) || !fourth()` in one return. This keeps the
original predicate identities and extends the compact representation from
[the discovery fixture](../guarded-boolean-return/) to nested and mixed
short-circuit decisions.

- A false first result skips the inner disjunction and reaches the fourth
  predicate. The fourth predicate's result is inverted.
- A true first result reaches the second predicate. A false second result
  accepts immediately, skipping the third and fourth predicates.
- A true second result reaches the third predicate. A true third result accepts
  immediately. A false third result reaches the fourth predicate and inverts
  its result.
- Every reached helper records its event before throwing the supplied exception
  when selected. The recovered text propagates that object without invoking
  any later helper. The original branch destinations and expression operands
  agree on whether each selected fault is reachable.

In `effectful()`, a true first result reaches `mark()` inside the guard. The
marker records event 5, may throw the supplied object, and otherwise reaches
the early true return. A false first result skips the marker and reaches the
second predicate, whose result is inverted. The 0.4.31 publication rewrites only
the second predicate's suffix. The marker is never reached by the first-false
lane or after failure in another helper.

Both candidates retain all original subject methods without recovery issues.
The [scenarios](scenarios.json) exercise normal and fault paths, call order, and
fault identity. [The ordinary Engine regression](../../../../crates/diana-decomp/tests/guarded_boolean_return_test.rs)
checks the nested grouping and the retained marker arm independently of the
executed fixture observations.
