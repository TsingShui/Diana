# Branched constructor values

This independently authored fixture extends the protected constructor paths in
[direct-initialization-effects](../direct-initialization-effects/README.md).
Each of two DEX allocations has two alternative direct calls to constructors
of its own class. Both initialized values reach a field read after their
branches join. The first value also reaches a typed handler when the second
initialization or the consumer fails. Reconstructing the constructors changes
the CFG instructions and their SSA definitions before the final Phi identities
are published.

The selector is `Lfixture/BranchedConstructorValues;->run(ZZ)I`.
The Java file explains the behavior. The Smali preserves the single allocation
site before each branch. `record` observes each initialized value before its
field store. The test model defines `consume(x,y)` as `100*x+y` and
`recover(x)` as `-x`. Each helper records its call before returning or raising
the injected exception.

| First short | Second short | Injected faults | Calls in order | Exit |
| --- | --- | --- | --- | --- |
| true | true | none | record(3), record(5), consume(3,5) | 305 |
| true | false | none | record(3), record(11), consume(3,11) | 311 |
| false | true | none | record(7), record(5), consume(7,5) | 705 |
| false | false | none | record(7), record(11), consume(7,11) | 711 |
| false | true | IllegalStateException at record(7) | record(7) | same exception |
| false | false | IllegalStateException at record(11) | record(7), record(11), recover(7) | -7 |
| true | false | IllegalStateException at consume(3,11) | record(3), record(11), consume(3,11), recover(3) | -3 |
| false | true | IllegalStateException at record(5), ArithmeticException at recover(7) | record(7), record(5), recover(7) | same ArithmeticException |
| true | false | ArithmeticException at record(11) | record(3), record(11) | same ArithmeticException |

## Preparation

The prepared DEX was assembled and disassembled at API 24 using Smali and
Baksmali 3.0.9 on OpenJDK 21.0.12.1. The original typed range begins at the
second allocation and ends after the consumer result.

```sh
fixture=quality/fixtures/exceptions/branched-constructor-values
work=.cache/fixtures/branched-constructor-values
mkdir -p "$work"
java -cp "$SMALI_CLASSPATH" com.android.tools.smali.smali.Main assemble --api 24 \
  -o "$work/classes.dex" "$fixture"
java -cp "$SMALI_CLASSPATH" com.android.tools.smali.baksmali.Main disassemble --api 24 \
  -o "$work/disassembly" "$work/classes.dex"
cp "$work/classes.dex" "$fixture/classes.dex"
diana get-class "$fixture/classes.dex" 'Lfixture/BranchedConstructorValues;' > "$work/class.java"
python3 "$fixture/check.py" "$work/class.java"
cargo nextest run --locked -p diana-decomp --test direct_initialization_test
```

The helper model in `check.py` replaces the three native methods and executes
the supplied Java body. The ordinary Engine test separately checks the original
method identity, constructor argument lists, protected extent and handler call.
