# String switch receiver identity

This independently authored JVM fixture uses ordinary Javac output. It tests
that recovery retains custom virtual dispatch even when the switch tables
resemble a compiler-generated String switch.

| Locator | Original member of `LStringDispatch;` | Original calls |
| --- | --- | --- |
| `j0:m1` | `custom(LSwitchKey;)I` | `SwitchKey.hashCode()I`, `SwitchKey.equals(Ljava/lang/Object;)Z` |
| `j0:m2` | `overloaded(LOverloadedSwitchKey;)I` | Inherited `OverloadedSwitchKey.hashCode()I`, two `OverloadedSwitchKey.equals(Ljava/lang/String;)Z` sites |
| `j0:m3` | `genuine(Ljava/lang/String;)I` | `String.hashCode()I`, two `String.equals(Ljava/lang/Object;)Z` sites |
| `j0:m4` | `genuineCollision(Ljava/lang/String;)I` | `String.hashCode()I`, two `String.equals(Ljava/lang/Object;)Z` sites under bucket 2112 |

`javap` checks two switch instructions and each original invocation's owner
and descriptor. The complex custom receiver inherits its hash method and
overloads equality. Its value `"BB"` shares bucket 2112 with the switch label
`"Aa"`, so the difference between custom equality and String comparison
affects the return as well as the order and count of virtual calls.

Each scenario starts with a fresh receiver and empty trace. Digit `1` is the
hash call. Digit `2` is the custom `equals(Object)` call. Digits `3` and `4`
are the overload with arguments `"Aa"` and `"z"`, respectively. A matching
custom equality returns 100 for `"a"` or `"z"`, and 200 for `"Aa"`. A
rejected or unlisted value returns 300. An unlisted hash skips equality.
A configured hash fault escapes before equality. A configured equality fault
escapes after its trace entry. The observer verifies the same exception
instance in both cases. A null receiver throws before any custom code runs.
The genuine controls cover matching, unmatched and null values, plus both
outcomes of one collision bucket.

The verifier runs the original class and then each exact selected publication
in isolation. The recovered classpath contains only that selected method,
the unchanged observer and the authored helper types. It records a definite
publication failure when a custom receiver is displayed with String case
labels and both original calls absent. A publication with another call shape
leaves that check unresolved. The unchanged observer judges the return,
original call sequence and fault identity when execution is available.

```sh
fixture=quality/fixtures/jvm/string-switch-receiver
work=.cache/fixtures/string-switch-receiver
python3 -B "$fixture/verify.py" --binary target/debug/diana --output "$work/proof.json"
python3 -B quality/decbench prepare --prepared "$work/prepared"
python3 -B quality/decbench run --prepared "$work/prepared" \
  --binary target/debug/diana --suite jvm-string-switch-receiver \
  --output "$work/decbench.json"
```

The original passes 21 runtime scenarios. Diana 0.4.24 reports no source
issues and drops both custom receiver dispatch protocols. DecBench records
two definite failures, 13 unchecked runtime scenarios and nine passing
behavior checks across the genuine controls, including compact recovery
for the ordinary String switch.

The 0.4.25 candidate passes all 36 DecBench checks on the same prepared
class. Both definite failures are resolved, and all 13 previously unchecked
runtime scenarios now execute and pass. The comparison gate reports zero
new failures and zero lost checks.
