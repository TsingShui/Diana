# Semantic review

`Terminal.self` examines its input first. A nonpositive input selects the
`seed` call, which adds seven. A positive input decrements once and
repeats `self` with that decremented argument. The two `bridge` calls
evaluate in left-to-right order with arguments `value` and `value + 1`.
`Relations.outer` relays their sum. The original `javap -c` listing
places the seed and recursive sites at bytes 5 and 14 of `j6:m2`, the
two `bridge` sites at bytes 1 and 7 of `j6:m3`, and the outer call at
byte 1 of `j1:m1`. The transitive query reports these five distinct
sites through three depths. The display cap still counts paths through
unshown rows.

`Relations.virtualDispatch` passes its receiver and the original integer
to the byte-2 `invokevirtual Leaf.transform` reference. `Leaf.transform`
uses its superclass implementation first and adds three to its result.
The outline links the encoded reference to the nested class declaration
without assuming the runtime receiver. `Relations.interfaceDispatch`
passes the same shapes to its byte-2 `invokeinterface Child.send`
reference. `Child` inherits the `Signal` default; `Leaf` supplies a
different implementation. The declaration links and implementation
witnesses preserve both possible bodies and their own locations.

The byte-1 `invokestatic Leaf.step` reference searches the original
`Leaf → Middle → Base` superclass path. `Base.step` is the first
matching declaration, and it subtracts two. The duplicate archive leaves
a `Terminal` linkage ambiguous. The unreadable class in the damaged
archive leaves the independently found `Base.step` body as a local
candidate and the original issue as a separate row.

The two branches through `Left` and `Right` reach `Common`. The
definition in `Right` dominates the older default in `Common` through
a known superinterface path. `Diamonds.read(new Winning())` therefore
returns two in the independent execution check. The outline records
`Right.pick` as the available declaration of the `Tip.pick` reference,
and the relationship query links the same original method to the
`Winning` receiver. The reviewed results concern the fixture's own
declarations and calls; inputs that supply other runtime definitions
can change a dispatch result.
