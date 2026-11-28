# Semantic review

The initial buffer is null. Each iteration reads a line before checking
the buffer. End of input exits without parsing and returns the buffer
available at the loop header. A partial first line becomes the new
buffer. A partial assembled line replaces the old buffer. The candidate
preserves these decisions and the order of the read, append and partial
calls.

For an assembled line the original parse has two typed catches. Normal
completion and a shape fault both reset the buffer. A bounds fault
reenters the read with the assembled value. The candidate preserves the
assembled value in a separate local and copies it back only in the bounds
catch. Its placement of the null assignment before the parse has no
observable effect on any other path. The shape catch sees null, the
bounds catch restores the value, and an unhandled fault exits the method
without reading the local again. The parse hook cannot access that local.

An unbuffered parse continues with null on normal completion and for
either matching catch. The candidate places the same null value on both
outcomes. Its multi-catch retains both original types. A parse fault of a
different type escapes before the next read. A bounds fault during the
append also escapes because append is outside of the parse protection.
The executed scenarios cover both escaping routes and check that a
retained line is used in a later append. An unresolved exception from
the read or partial hook would also escape in both bodies. Those hooks
were reviewed by inspecting their placement and were not made to throw
during model execution.

The release baseline provides no method body for any of these paths.
