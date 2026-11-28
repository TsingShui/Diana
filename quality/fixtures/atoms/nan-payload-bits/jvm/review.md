# JVM NaN payload review

The original JVM instruction reads each value from a floating constant-pool
entry. The first two methods return it directly. The joined selectors choose
a distinct constant on each path before the original raw-bit observer call.
Diana 0.4.21 publishes canonical NaN in all six cases even when the original
entries have different payload and sign bits. The patched class uses
`java.lang.Float.intBitsToFloat` and `java.lang.Double.longBitsToDouble`
with the original bits. The same compiled witness observes both original
payloads and sign bits on each path. The negative zero and positive infinity
controls retain their original bits.

Both `NaNChild` fields are assigned by the JVM from `ConstantValue` before
its parent's initializer. The original superclass reads `7fc01234` and
`fff8000000005678` during that initialization. The explicit runtime assignment
control has no `ConstantValue` on those fields and its compiled `<clinit>`
performs the writes later. The superclass reads zero bits, while a later
read observes the original NaN bits. This distinguishes preparation timing
from the final stored values.

The baseline declares `Float.NaN` and `Double.NaN`, losing the exact
prepared payloads. The patched publication uses a Java-like field literal
that retains each original bit pattern and identifies the `ConstantValue`
origin. It has no executable static initializer. The field publication is
checked as display evidence. The initialization timing check executes the
original class and the authored runtime-assignment control.
