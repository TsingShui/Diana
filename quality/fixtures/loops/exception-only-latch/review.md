# Semantic review

The Smali header exits to `empty` for zero and negative counts. When the
count is positive, every ordinary successor of the switch returns from the
method. Only a caught `IllegalStateException` reaches the decrement and the
edge back to the header. Diana's ordinary class retrieval renders this as
a while loop surrounding one try and catch. Keys 0 and 1 fall through to
the same return. The emitted nested call passes the result of `first`
directly to `second`. Key 2 calls only `alternate`, and the default calls
only `stop`. Each arm returns the value from its selected outermost call.

The try surrounds `choose` and all arm calls. A caught failure in `first`
skips `second`. A caught failure in `second` happens after the effects of
`first`. Both reach the same retry with the original count. Successful
retry decrements once before the next condition test. The call to `retry`
is in the handler body, so its own `IllegalStateException` escapes. A
different type thrown by a protected call also escapes immediately. The
final `empty` call is outside protection and receives the exact value
that failed the positive-count test.

An independent JVM run replaced the native declarations in the Java
explanation and in the retrieved source with identical recording helpers.
It compared ordered helper identities and arguments, return values, and
the identity of any escaping exception across 20,160 input and fault
combinations. Counts ranged from -2 to 7 and switch keys from -2 to 4.
Failures varied by helper, current count and exception type. A separate
fault could make retry throw the caught type. All comparisons agreed.
The harness runs single-threaded and does not model asynchronous
exceptions or runtime linkage failures.

The ordinary JSON output was byte-identical before and after the change
for this class and 14 classes in the existing loop, branch and exception
fixtures. The separate engine regression retrieves the subject by original
class, name and DEX prototype and checks the switch, handler, decrement
and final return in their lexical order.
