# Recursive throws review

The ordinary class retrieval produced identical source before and after the
incremental exception-contract change. I compared the following paths with
the Smali and the independent expectations.

- The false arm of `c` invokes the native operation and returns immediately on
  success. No handler intercepts an exception there. The true arm calls
  `b(false)`, which reaches the false arm on its next call to `c`. This
  preserves termination of the recursive path and the original boolean values.
- `b` covers its invocation with exactly the input-defined `ReadFailure`
  catch. The handler does nothing and falls out of the method. The calls in
  `a` and the true arm of `c` propagate any other throwable from that path.
  The declarations of all three methods retain `throws Throwable`.
- The first call in `d` has its own `ReadFailure` catch. A matching failure
  falls through to the second call. Any other failure escapes before the
  second call. The second call has a separate `Throwable` catch. The
  declaration of `d` retains the possible first-call escape.
- The protected call in `e` encloses all of `d`, and the catch-all has an
  empty body. `e` ends without a checked throws clause. The native declaration
  retains its DEX throws annotation in the emitted signature.

The comparison used `get-class` on the prepared DEX. The behavior of the
native operation is an explicit input assumption. No native implementation was
executed.

The ordinary Engine regression also passed during the combined nextest run.
It selects the original class descriptor and checks all six declarations.
