# Typed catch return across cleanup

These independently authored sources reduce a return-path discrepancy observed
during full public corpus evaluation. The DEX method is
`Lfixture/ReturnLanes;->lookup(Ljava/lang/Object;Ljava/lang/RuntimeException;Ljava/lang/RuntimeException;Ljava/lang/RuntimeException;)Ljava/lang/Object;`
at `e0:l0:c0:m1`. The caught exception is ignored. Its handler closes the
owner, releases the class monitor, and returns null. The successful path
returns the captured input after the same cleanup effects.

The Smali places the owner-unwind block before the ignored typed handler.
Moving the two blocks without changing their edges provides a layout
control. Diana 0.4.29 retained the null return in the alternate layout.
In the tracked layout it fell through from the catch to the captured input.

The [expected scenarios](scenarios.json) include successful and empty returns,
body and cursor-close failures, suppression identity, self-suppression, and
owner-close failure on each kind of completion. Event 1 opens the monitored
body, event 2 captures the item, event 3 represents the later body work, and
events 4 and 5 close the cursor and owner. The observer checks object identity,
the ordered effects, the original suppressed object, and monitor release.
The Java observer entry points are test support. The Java `lookup` and its
work helpers explain the Smali method.

The prepared DEX was assembled with Apktool 2.11.1's embedded Smali at API 24.
An equivalent assembly with a local Smali installation uses the following
commands from the repository root.

```sh
fixture=quality/fixtures/exceptions/typed-catch-cleanup-return
work=.cache/fixtures/typed-catch-cleanup-return
mkdir -p "$work/smali"
cp "$fixture/ReturnLanes.smali" "$work/smali/"
java -cp "$SMALI_CLASSPATH" com.android.tools.smali.smali.Main assemble --api 24 \
  -o "$work/classes.dex" "$work/smali"
python3 -B quality/decbench run --binary target/debug/diana \
  --suite dex-typed-catch-cleanup-return --output "$work/decbench.json"
```

The behavioral probe compares the Java explanation and the selected Diana
publication using the same recording helpers. The original-order control
and the authored Java each matched all ten scenarios. The DEX treatment and
the monitor depend only on original input facts. The behavioral runs execute
the compiled Java probes.

The [late-effect challenge](../typed-catch-cleanup-return-late-effect/) adds
one throwing action between cursor close and owner cleanup.

Diana 0.4.29 passes six of the ten behavioral scenarios on the tracked
layout. Diana 0.4.30 passes all ten.
