# Operation ownership and phase review

LLM review compares authored Java/Smali and ordinary dbg_shell output on the
retained `76f19c8` binary and the current operation-record replacement. Original
operations, member declarations and copy facts remain available. The Rust
selected-plan subset executes the explicit helper/lock model; no real native
helper, allocator, lock or JVM is executed.

In protectedNew, first captures its original operands and saves the handler's
zero value before invoking F. A first fault therefore returns zero. Normal F
completion commits its result before allocation and later. P5 selected a common
snapshot prelude whose source is that committed value at the allocation anchor.
The baseline attached it to the source-only allocation's snapshot list, which
the selected New body never executed. Consequently later/gate faults read the
stale zero. This contradicts the input's saved-value behavior.

The new selected phase attaches that delivered prelude before the initializing
operation's entire New evaluation. It saves the committed value before allocation
and before L, preserving P5's closed-corridor and available-source guarantees.
Allocation failure still escapes as OutOfMemoryError before L; a later fault skips
G; a gate fault preserves reached effects. Both RuntimeException paths read saved,
and the success-only increment runs only after construction succeeds. Body data
remains owned by the allocation coordinate; that coordinate is not an execution
schedule. Restoring the old owner is detected as 0 versus expected 10 on L's fault.

In locked, the original acquire precedes F. Null fails before acquiring or
invoking helpers. A negative F result skips L, while a nonnegative result reaches
L. The supplied implicit synchronized scope performs one release on every
acquired result or helper-fault path; source-only compiler releases do not execute
again. The original RuntimeException then propagates without replacing its kind.
The new body/phase representation preserves this qualified lock correspondence.

RecordBox retains its original super delegation before gate. The original-op
prefix, not the now-prepared complete metadata table, establishes first-body
position. Constructor arguments and helper parameters retain their input order.

All three code members and all three native declarations remain retrievable.
No contradiction was found on the listed paths after the phase fix. Twenty
focused Rust checks pass, with three checks from this challenge. Model support is
limited to its scalar/reference, constructor, copy, exact catch and implicit lock
subset; unsupported features are errors, not passing paths. VM/device execution,
the general final-plan executor and historical regression remain unrun/deferred.
