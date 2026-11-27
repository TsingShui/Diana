# Exact JVM lambda delegates

These independently authored inputs use OpenJDK's compiler for Java 17 class
files. `SyntheticLambda.arithmetic` and `SyntheticLambda.action` each carry
a noncapturing `LambdaMetafactory.metafactory` site. The implementation handle
names a private synthetic static helper declared in `SyntheticLambda`.
The call-site SAM type, instantiated SAM type and helper signature agree.

The arithmetic function takes an integer and evaluates `value * 3 + 7`.
The `Runnable` defers a write to `trace` until invocation. Two invocations
append 5 twice. The check runs the original class and the recovered Java
compiled separately. The source comparison also requires the exact helper
names and both retained bodies.

`CapturedLambda.captured` snapshots an immutable integer parameter through a
local copy and captures a category-two long after it. `joined` preserves one
entry value across two arms while their distinct trace writes remain eager.
Separate invocations on several captures check the bound values and trace order.
The execution test disambiguates the original helper names from the compiler's
new synthetic names before recompiling. The source assertions retain the original
helper identities.

`AdaptedLambda.trim` narrows an erased `Object` argument to `String`
and returns a `String` through an `Object` result slot. `join` narrows two
arguments to `String` and `String[]` while preserving its original immutable
capture. The raw-interface execution check passes incompatible arguments in
each position and observes `ClassCastException` before either helper writes
its trace. Both factories execute before invocation without writing the
trace. Subsequent valid calls establish the helper arguments and effect order.

`UnsupportedCapture` establishes three remaining evidence boundaries. The call to
`read` has an eager side effect, another original parameter is overwritten, and
a conditional join can select between different parameters. The original source
executes independently. Recovered source keeps the eager operations, computed
operands and exact call-site diagnostics.

```sh
cargo nextest run --locked -p diana --test jvm_lambda_test
```

The fixture uses the [LambdaMetafactory linkage and capture contract](https://docs.oracle.com/en/java/javase/21/docs/api/java.base/java/lang/invoke/LambdaMetafactory.html).
