# Nested guarded boolean return

This independently authored challenge uses the Diana repository license and
extends the [guarded boolean return fixture](../guarded-boolean-return/).
The selected methods are `Lfixture/MixedBoolean;->mixed()Z` at
`e0:l0:c1:m7` and `Lfixture/MixedBoolean;->effectful()Z` at
`e0:l0:c1:m6`.

In `mixed()`, a false first predicate skips both inner tests and reaches
the fallback. A true first predicate reaches the second test. Its false
result shares an accepting return with the true result of the third test.
Otherwise execution reaches the fallback, where a false fourth test also
accepts. In Java this is
`first() && (!second() || third()) || !fourth()`.
The Smali retains explicit branches and both accepting and rejecting exits.

The `effectful()` control runs `mark()` on the first-true arm before returning
true. The second predicate runs only on the first-false arm. The marker
appends event 5 to the trace and may throw the original supplied exception.
Both actions use the same trace recording and fault rules as the parent
fixture. This arm provides a preservation check for an observable action
between a guard and its constant return.

Each [scenario](scenarios.json) supplies the subject, flags and fault selector.
The observer starts with trace zero and a fresh exception object. The twelve
mixed scenarios cover the nested accepting paths, the fallback from both
routes, faults at each predicate and configured faults skipped by earlier
decisions. The six effectful scenarios cover reachability of the marker
and second test, their results, their failures, and cases where the other
arm's fault is skipped.

Diana 0.4.30 combines the inner tests of `mixed()` but publishes the result
as two guarded constant returns and a final true return. It retains the
marker and path in `effectful()`. Bounded replay of the original DEX and
execution of the authored Java and exact frozen Diana publication each
match all eighteen scenarios. The DecBench manifest checks the recovered
mixed return separately from these behavior checks.

Diana 0.4.31 emits the nested expression in one return and passes the three
publication checks that fail on 0.4.30. Both releases pass all eighteen
executed scenarios. Eager evaluation in `mixed()` and omission of the marker
in `effectful()` each cause the corresponding behavioral probe to fail.
See the [semantic review](review.md).

The DEX was assembled with Apktool 2.11.1's embedded Smali, API 24, on
OpenJDK 21.0.12.1, using `SmaliBuilder.buildFile` and
`DexBuilder(new Opcodes(24, -1)).writeTo`. An equivalent local Smali recipe and
the focused checks can run from the repository root.

```sh
fixture=quality/fixtures/branches/nested-guarded-boolean-return
parent=quality/fixtures/branches/guarded-boolean-return
work=.cache/fixtures/nested-guarded-boolean-return
mkdir -p "$work/smali"
cp "$fixture/"*.smali "$work/smali/"
java -cp "$SMALI_CLASSPATH" com.android.tools.smali.smali.Main assemble --api 24 \
  -o "$work/classes.dex" "$work/smali"
PYTHONPATH=quality/decbench python3 -B "$parent/verify_raw.py" "$fixture"
python3 -B quality/decbench run --binary /path/to/diana \
  --suite dex-nested-guarded-boolean-return --output "$work/decbench.json"
```
