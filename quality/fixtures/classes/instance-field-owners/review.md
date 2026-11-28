# Semantic review

A DEX instance-field operand includes a declaring owner, a name and a
descriptor. The authored hierarchy deliberately puts three independent
int slots named `value` on each receiver. An `iget` or `iput` selecting
the parent or root declaration must access that particular slot. The
original peer write at code unit 0 affects the supplied object's root
slot before the read of the current receiver's root slot at 2. The two
receivers may alias. The observed return value and both snapshots
distinguish the alias and distinct paths.

Diana 0.4.17 renders all hidden field references through the
most-derived surface. This returns the wrong value for the ordinary
reads and writes a different slot. A normal run with an aliased
receiver can still return the written number. The independent snapshots
detect that the operation updated the derived slot.

The repaired engine uses `super` for the lexical direct parent and a
receiver cast for the root declaration. Its peer write and later read
retain both distinct receivers and exact slots. The arbitrary receiver
producer runs once and resolves the root slot from the returned
reference. If that reference is null, its counter update remains
visible before the null field access fails. The authored same-owner
and null scenarios continue to agree with the original.
