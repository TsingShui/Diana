# Predicate words and integer loop values

DEX `instance-of` yields the numeric word zero or one. These methods
use that word as an initial value and join it with ordinary integers.
`select` can take an unrelated negative value. `run` carries the initial
word across a loop, increments it on selected iterations, and tests the
result for zero. The native declarations in Smali let the Java model record
the integer arguments and zero branch.

The input was assembled into `.cache/fixtures/predicate-word-join/classes.dex`
with the Smali builder in [Apktool 2.11.1](https://github.com/iBotPeaches/Apktool/releases/tag/v2.11.1)
on OpenJDK 21.0.12.1, API 24. It used `SmaliBuilder.buildFile` and
`DexBuilder(new Opcodes(24, -1)).writeTo`.

Diana 0.4.2 at `49af4a17` emits a Boolean local for `run` and converts the
integer backedge to a nonzero flag. The model cannot execute those five
baseline scenarios because the recovered Java-like arithmetic on a Boolean
fails to compile. The working-tree build keeps an integer local and passes
all eight model scenarios across the two methods.

[Scenarios](scenarios.json) · [Semantic review](review.md)

```sh
python3 quality/decbench prepare
python3 quality/decbench run --suite dex-predicate-word-join --binary /path/to/diana
```
