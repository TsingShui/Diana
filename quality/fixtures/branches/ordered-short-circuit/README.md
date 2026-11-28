# Ordered short-circuit return guards

This independently authored challenge covers the shared return in the
[FairEmail source method](https://github.com/M66B/FairEmail/blob/a239b8bf3f4599b2f0ad27795c13ff97e0f4f4cd/app/src/main/java/eu/faircode/email/ConnectionHelper.java#L519-L530).
The original member is
`Lfixture/OrderedShortCircuit;->run(Ljava/lang/String;)Z`.
The left and right calls have distinct true return blocks in Smali.
The null check and loop advance survive independently of those blocks.
The fixture makes each predicate and the advance observable and makes
either predicate throw. The [scenario table](scenarios.json) covers null
input, both possible accepting routes, exhaustion, a second iteration,
and both fault sites. Smali uses native declarations. The Java explanation
supplies the recording behavior under the model.

The DEX was assembled into `.cache/fixtures/ordered-short-circuit/classes.dex`
with the Smali builder in [Apktool 2.11.1](https://github.com/iBotPeaches/Apktool/releases/tag/v2.11.1)
on OpenJDK 21.0.12.1, API 24. It used `SmaliBuilder.buildFile` and
`DexBuilder(new Opcodes(24, -1)).writeTo`.

Diana 0.4.2 at `49af4a17` produces two guarded true returns. The
working-tree build at the same base combines the two calls with short-circuit
OR under the same null guard. Both bodies pass the seven executed scenarios.
The engine fixture test additionally requires the combined guard.
[Semantic review](review.md)

```sh
python3 quality/decbench prepare
python3 quality/decbench run --suite dex-ordered-short-circuit --binary /path/to/diana
```
