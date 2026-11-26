# Bounded array initializer review

The pre-fix and candidate are local builds of checkpoint `a846548` before and
after the aggregate-initializer change. Both read the same prepared DEX through
`get-class`. On 64-bit Linux with a 512 MiB address-space limit, the pre-fix
request aborted while reserving 326417514344 bytes. The candidate returned all
three methods with status zero under the same limit.

For `partial`, Smali allocates before writing. The argument supplies only
element zero, so treating that store as a complete initializer would give
a different array length. The candidate emits the full length, followed by
the index-zero write and return of the same array. If the allocation throws,
control cannot reach the store or return in either description.

For `complete`, the original writes the first argument to element zero and
the second to element one. The candidate puts these arguments in that order
in a two-element initializer and returns the allocated array. For `dependent`,
the second store reads index zero of the new array after the first store.
The candidate keeps the allocation and both stores in order. This retained
read distinguishes the dependent case from two independent stores, even
though both methods fill every element. The earlier small-array allocation
and ordered index access in the [discovery fixture](../array-basic/) remain
covered by the same Java lowering.

These comparisons review the recovered paths from their DEX instructions.
The bounded run exercises Diana's allocation behavior. Execution of the
represented Android methods was outside that run.
