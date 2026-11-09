# Cast carriers across ordered contexts

Independently authored Java and Smali under the repository Apache-2.0 license.
Original owner: `Lfixture/CarrierContexts;`. This is the complexity challenge for
[cast carrier order](../../atoms/cast-carrier-order/README.md), on baseline
`dd3a114` and the containing shared-walk refactor. Expectations precede inspection.
The engine receives DEX alone, without external declarations.

`triplet(Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;)I` checks the
middle reference as int[], then tests a == b and b == c in that order. The second
comparison is skipped when the first fails. The cast executes once even when
that first comparison fails. Aliasing a and b does not establish the cast type.

`caught(Ljava/lang/Object;Ljava/lang/Object;)I` performs the right-operand check
and comparison in a ClassCastException try range. It returns -1 on that exact
cast failure, 1 for equal references and 0 for different references. There is no
helper or catch effect, and no retry. `acting(Ljava/lang/Object;)I` checks its
input, calls make, then compares the returned reference with the checked input.
make records M, returns a supplied reference and may throw E. A failed cast
prevents M; the check cannot move into the comparison after make's evaluation.
Class initialization is assumed established for the modeled helper scenarios.

Use null, int[] A, distinct int[] B, and incompatible object O as in the parent.
Triplet returns 1 for (A,A,A) and (null,null,null), 0 for (A,B,B), (A,A,B) and
(A,null,null). A middle O throws ClassCastException before either comparison.
Caught(A,A) and caught(null,null) return 1; caught(A,B) returns 0;
caught(A,O) returns -1. Acting(A), make returns A, returns 1; make returns B,
returns 0. Acting(null), make returns null, returns 1. Acting(O) throws before M.
Acting(A), make faults, throws exact E after M. No native helper or VM execution
is claimed by these written expectations.

Required preservation: exact checked value/state, one original check/site,
short-circuit evaluation count, catch membership and effect order. Comparison
and operation names must not select a recovery path. Historical fixtures remain
deferred at the operator's request.

```sh
mkdir -p .cache/fixtures/cast-carrier-contexts
java -cp '.cache/evolution-demo/smali-tools/*' org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/cast-carrier-contexts/classes.dex quality/fixtures/composite/cast-carrier-contexts
cp .cache/fixtures/cast-carrier-contexts/classes.dex quality/fixtures/composite/cast-carrier-contexts/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/composite/cast-carrier-contexts/classes.dex \
  --exec '{"cmd":"p9-all","text_only":true}'
```

## Observed checks

Prepared in the parent's Smali 2.5.2/API 24/OpenJDK 21.0.12.1+1 environment.
Native selector: make()Ljava/lang/Object;. Both baseline and candidate retain
four methods, three executable results and no member or render failure. Triplet
improves its first-test carrier; caught and acting retain their exact text.

The parent and challenge's six nextest cases execute 41 independently specified
reference scenarios: 27 normal exits and 14 escaping faults, including exact
helper effect/argument traces. Catch recovery is executed on the incompatible
input scenario. Together with the new joint-expression-cover cases, the scoped
nextest run has eleven passes and no skips. Library/example/two-test Clippy with
all engine features passes, and the dev dbg_shell build passes. The integer and
reference checks use one test-only selected-product executor. No historical
fixture run, release duplicate, external declaration lookup or VM run occurred.
