# Shared protected work reached through several handler families

A first protected operation either returns or enters one of two typed
handlers. Both handlers perform a private observable action and reach a
second protected operation. Its normal completion returns. Its two typed
handlers perform distinct actions before converging at a third protected
operation. The last operation has its own interrupted-exception handler.
None of the four preceding handlers individually owns that shared work.

`runRepeated` forms a loop around the last operation and its handler.
It adds the caught-exception edge back into the loop and distinguishes a
normal early return, interrupted iterations, and a later unmatched fault.
The Smali uses native helpers. The Java control supplies trace recording
and asserts that each handler receives the exception object actually
thrown at its corresponding call.

The input was assembled into `.cache/fixtures/multiple-handler-families/full.dex`
with the Smali builder in [Apktool 2.11.1](https://github.com/iBotPeaches/Apktool/releases/tag/v2.11.1)
on OpenJDK 21.0.12.1, API 24. It used `SmaliBuilder.buildFile` and
`DexBuilder(new Opcodes(24, -1)).writeTo`.

[Scenarios](scenarios.json) · [Semantic review](review.md)

```sh
python3 quality/decbench prepare
python3 quality/decbench run --suite dex-multiple-handler-families --binary /path/to/diana
```
