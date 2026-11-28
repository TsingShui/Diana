# Catches that share an ordinary loop continuation

A buffered line and an unbuffered line both return to the same loop
header. Two separately protected calls can reach that header on an
exception. The first call keeps its assembled line on a bounds exception
but discards it on a shape exception. The other call discards the value
for either exception. An ordinary edge also reaches the same header.

Diana 0.4.2 at `49af4a17` reports an unavailable exceptional Phi copy
and omits the body. The working-tree build keeps both catch occurrences
and passes all nine model scenarios. The model records calls, faults,
the buffer returned at end of input, and the value read by the next
iteration. The Smali uses native helpers and the Java source provides
their deterministic model behavior.

The input was assembled into `.cache/fixtures/exception-loop-ingress/classes.dex`
with the Smali builder in [Apktool 2.11.1](https://github.com/iBotPeaches/Apktool/releases/tag/v2.11.1)
on OpenJDK 21.0.12.1, API 24. It used `SmaliBuilder.buildFile` and
`DexBuilder(new Opcodes(24, -1)).writeTo`.

[Scenarios](scenarios.json) · [Semantic review](review.md)

```sh
python3 quality/decbench prepare
python3 quality/decbench run --suite dex-exception-loop-ingress --binary /path/to/diana
```
