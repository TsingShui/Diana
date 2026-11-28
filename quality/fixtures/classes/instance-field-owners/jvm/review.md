# JVM instance field owner review

The original JVM class retains field owners in both `getfield` and `putfield`.
`readParent()` observes the parent slot, initially 37. `writeParent(49)`
leaves the child slot at 5. In `writeThenReadBoth(11)` the child slot is
incremented to 7 and the two original slots produce 1107. Frozen 0.4.17
uses `this.slot` for both owners. It reads 5 initially and changes only the
child slot on a parent write. The mixed update changes that same slot twice
and returns 1313. The current repair uses `super.slot` for accesses to the
parent declaration and `this.slot` for the child declaration. Its direct
super method invocation already agreed with the original in both binaries.

The separate `FieldPeer` class retains the same parent field reference,
but the receiver expression is a call returning `FieldChild`. The baseline
calls the helper only once and then uses the child's field. The repaired
publication qualifies the result with `FieldParent`, and recompilation
retains both the call and the parent field target. Success reads and writes
change only the selected object's parent slot. Both null paths still run
one receiver evaluation then throw before any field value is returned or
written.

The raw recompiled target sequences and the externally observed values agree
with the original on both subjects. This covers two classes declaring the
same field name and descriptor, through a lexical superclass access and an
external effectful receiver.
