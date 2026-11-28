# Semantic review

The FairEmail APK contains a related case in the bundled AndroidX
`Selection.toString()`. DEX code unit `0xb` invokes `Selection.size()` a
second time, multiplies its result by 28, and supplies it to
`StringBuilder.<init>(I)V` at `0x11`. Diana 0.4.10 drops the second call and
returns a concatenation. The candidate keeps the capacity computation in
`new StringBuilder(this.size() * 28)` ahead of the appends. The second
virtual call can independently return a negative value or throw. The APK
comparison checked the original instruction and recovered call order.
[Pinned FairEmail source](https://github.com/M66B/FairEmail/blob/a239b8bf3f4599b2f0ad27795c13ff97e0f4f4cd/app/src/main/java/androidx/recyclerview/selection/Selection.java#L216-L228)

In all three originals, the builder or buffer capacity is validated
before the caller reaches either producer. A negative size therefore
throws without a producer event. In the protected method the original
exception is caught by the capacity arm. The unprotected methods let
that exception escape.

Diana 0.4.10 renders all three chains as string addition. It keeps the
source-shaped handler headers in the protected method, but the
replacement expression omits the capacity validation. The negative
cases execute the modeled producers, with their failures reaching
an outward caller or the producer catch instead.

The candidate restores the original capacity constructor before the first
append expression, including inside the two-handler try region. When
construction throws, the candidate skips both producers. The protected
method delivers the exception to the capacity handler. In the other methods
the exception escapes.

For successful capacities, the baseline and candidate evaluate the hooks in
original order, append both strings, and return their concatenation.
The first producer fault skips the second producer. The second
producer fault follows the first successful producer. The observed
throwable object is preserved in both versions. The deliberately
unmatched fault bypasses both typed catches as expected.

The executed checks compare the retained DEX invokes and typed handler
targets with authored library models and run the recovered methods with
the JDK implementations. Allocation limits and resource exhaustion remain
outside the fixture's executed scenarios.
