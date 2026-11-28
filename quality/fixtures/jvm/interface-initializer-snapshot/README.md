# JVM interface initializer snapshots

These independently authored interfaces compile with ordinary Javac. Each
initializer's switch expression introduces a local without a helper method,
so the instructions under test run inside the interface's own `<clinit>()V`.
The verifier targets JVM 17 and uses a fresh process for each initialization.

| Original member in the prepared JAR | Original snapshot and subsequent effects |
| --- | --- |
| `LInterfaceSnapshot;-><clinit>()V` (`j0:m0`) | Read `SnapshotEffects.INSTANCE` once, replace it, assign the saved reference to `RESULT` |
| `LProtectedSnapshot;-><clinit>()V` (`j1:m0`) | Read once before the typed exception range, replace, optionally catch the original fault and observe the saved reference, then assign the saved reference |
| `LStableSnapshot;-><clinit>()V` (`j2:m0`) | Trigger initialization of `StableOwner`, read its immutable `INSTANCE`, and preserve its identity through the replacement and the result |

`SnapshotEffects.INSTANCE` is volatile and starts as `BEFORE`. The replacement
method appends trace digit 1 when it sees the original instance and sets
the field to `AFTER`. The protected fault scenario throws the configured
exception immediately after this write. Its handler appends digit 2 for a
saved `BEFORE` reference and digit 4 for the exact configured exception.
The stable field's owner appends digit 3 on initialization before the call
that appends digit 1.

| Scenario | `RESULT` is `BEFORE` | Mutable field is `AFTER` | Trace |
| --- | --- | --- | --- |
| Basic capture | true | true | 1 |
| Protected normal | true | true | 1 |
| Protected fault | true | true | 124 |
| Stable field control | true | true | 31 |

`javap` verifies one original `getstatic` of `INSTANCE` before the replacement
and one `putstatic RESULT` after it. The protected method's exception table
covers the call while the capture precedes the protected range. The verifier
recompiles the exact full class publications. The same previously compiled
observer and helper classes run against both sets of interface classes.

```sh
fixture=quality/fixtures/jvm/interface-initializer-snapshot
work=.cache/fixtures/interface-initializer-snapshot
python3 -B "$fixture/verify.py" --binary target/debug/diana --output "$work/proof.json"
python3 -B quality/decbench prepare --prepared "$work/prepared"
python3 -B quality/decbench run --prepared "$work/prepared" \
  --binary target/debug/diana --suite jvm-interface-initializer-snapshot \
  --output "$work/decbench.json"
```

The original JVM classes pass all four runtime scenarios. Diana 0.4.25
repeats the volatile field access two or three times, giving the new value
to `RESULT` in all three mutable scenarios. In the protected fault case the
same exception still reaches the observer, but with the wrong saved object
and trace 194. The stable field control passes with trace 31.
DecBench records three definite behavioral failures and ten passes,
including original member selection, publication, and diagnostics.

The Diana 0.4.26 candidate passes all four runtime scenarios and all
13 DecBench checks on the same prepared JAR. The comparison gate reports
three resolved failures and no loss of evidence.
