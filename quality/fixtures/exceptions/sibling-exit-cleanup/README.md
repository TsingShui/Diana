# Sibling control exits and cleanup

These independently authored subjects extend
[shared switch tails](../../branches/switch-shared-tails/README.md) and
[branching finally suffixes](../branching-finally-suffix/README.md).
Their original identities are `Lfixture/SiblingExit;->run(II)I` and
`Lfixture/SiblingExit;->sweep(III)I`. The fixture models native
`Lfixture/ScopeHooks;` calls as observable events that can return or throw.

`run` puts a switch and a countdown inside a protected outer loop. Key 0
skips the countdown, keys 1 and 2 share the countdown and read, and the
default returns immediately. The countdown finishes before the following
typed catch becomes active. An `IllegalArgumentException` from the read
is recoverable. The same type thrown by the preceding countdown escapes
through the outer cleanup. Cleanup runs once on a successful return or
a pending exception and may replace that exception.

`sweep` finishes a switch and optional countdown before entering an
independent protected row loop. An exception in this first switch escapes
without running the row cleanup. Each row can recover a
`NumberFormatException` or `NullPointerException` from its read before
adding that row's value and continuing. The enclosing cleanup depends on
`mode` and can throw from its first or second operation.

The Java explanation expresses the same transfers with for loops. The
Smali shares switch targets and fixes the boundaries of the typed and
catch-all handlers. The Java hook model in `Check.java` records the
ordered helper names and arguments. Its twelve independently specified
scenarios check the final result or exact escaping exception object.
The model assumes single-threaded execution.

The prepared DEX was assembled with Smali 3.0.9 and OpenJDK 21.0.12.1 at
API 24. The ordinary Engine regression runs without an assembler or JVM.

```sh
fixture=quality/fixtures/exceptions/sibling-exit-cleanup
work=.cache/fixtures/sibling-exit-cleanup
mkdir -p "$work/jvm"
java -cp "$SMALI_CLASSPATH" com.android.tools.smali.smali.Main assemble \
  --api 24 -o "$work/classes.dex" "$fixture"
cp "$work/classes.dex" "$fixture/classes.dex"
cargo nextest run --locked -p diana-decomp --test sibling_exit_cleanup_test
diana get-class "$fixture/classes.dex" 'Lfixture/SiblingExit;' > "$work/SiblingExit.java"
java -m jdk.compiler/com.sun.tools.javac.Main -d "$work/jvm" "$work/SiblingExit.java" "$fixture/Check.java"
java -cp "$work/jvm" fixture.Check
```
