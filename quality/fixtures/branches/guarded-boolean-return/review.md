# Guarded boolean return review

Diana 0.4.30 emits an early true return after `first()`, an early false return
after `second()`, and a true fallback. Diana 0.4.31 publishes one return with
`BooleanEffects.first() || !BooleanEffects.second()`. Both versions recover the
same original method without recovery issues.

The raw DEX skips the second invocation whenever the first test returns true.
The left operand of the recovered disjunction does the same. When the first test
returns false, both forms run the second predicate and invert its result. No new
value evaluation is needed after either predicate. This reasoning covers all
normal paths, independent of a particular sequence of flags.

Each helper appends its event to the trace before checking the fault selector.
An exception from the first helper preserves trace 1 and prevents the second
call. When the first result is false, an exception from the second helper
escapes with trace 12. Neither form catches, replaces, or delays that exception.
A fault selected for a skipped helper cannot occur.

The independently authored [scenarios](scenarios.json) test those effects and
exact exception identity. [The ordinary Engine regression](../../../../crates/diana-decomp/tests/guarded_boolean_return_test.rs)
checks the compact representation as a separate readability criterion. It does
not execute the recovered Java.

The originating AntennaPod method at `e4:l0:c8043:m54292` also changes from two
guards and three returns to an ordered disjunction of the original calls. The
fixture substitutes an observable predicate pair for application dependencies.
[Pinned AntennaPod source](https://github.com/AntennaPod/AntennaPod/blob/175e5ef8fe3e125a5014d70ef0c57cf85661c71b/net/common/src/main/java/de/danoeh/antennapod/net/common/NetworkUtils.java#L43-L45)
