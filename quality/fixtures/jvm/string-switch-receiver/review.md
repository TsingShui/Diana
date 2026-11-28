# String switch receiver identity review

The original `custom` dispatches on `SwitchKey.hashCode()`. Only bucket 97
calls `SwitchKey.equals(Object)`. The cast-free String argument reaching
that method carries no guarantee about the receiver's equality semantics.
For the normal `"a"` case, the same hash and equality sequence yields 100
when accepted and 300 when rejected. The hash fault preserves trace `1`,
the equality fault preserves trace `12`, and an unlisted hash with a pending
equality fault returns 300 and trace `1`.

`overloaded` calls the inherited hash behavior through its subclass and
selects `equals(String)`. In bucket 2112 a `"BB"` receiver may accept the
different `"Aa"` label. That path returns 200 and records `13`, or records
`13` and propagates the exact configured equality fault. The `"z"` path
records `14`. This distinguishes primitive hash selection, the inherited
call, the overload and equality's own return.

Diana 0.4.24 renders the two custom receiver methods as String-labeled
switches and removes the original calls. That publication omits the
receiver effects and exception sites on every reached call path. Java source
recompilation cannot start the unchecked execution scenarios for those
methods. Both genuine controls execute as expected after recovery. The
noncolliding String switch is reconstructed directly. The colliding control
retains both String comparisons and distinguishes `"Aa"` and `"BB"`.

The 0.4.25 candidate keeps both custom methods' original calls. Their
recompiled publications preserve each return, trace and exact fault identity,
including the early null failure. The ordinary String control retains its
compact switch, and the genuine collision control keeps its two distinct
comparison paths. The unchanged runner passes all 21 runtime scenarios.
