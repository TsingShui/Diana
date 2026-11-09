# Shared region membership with interleaved source portions

Independently authored Java/Smali under the repository Apache-2.0 license.
Original owner: `Lfixture/MemberForest;`. This challenges
[fragment ordering](../../branches/fragment-order/README.md) and its
[loop/switch extension](../fragment-order-loops/README.md) against `02831ff`.
The replacement removes ancestor member vectors. It derives one leaf permutation
and region ranges from the installed child ownership, without changing original
links, entries, copies, protection, locks or execution children.

The first method physically interleaves the portions of its two private arms.
The second adds typed catch completion to a guarded loop and aliased switch
bodies. The third puts a loop and throwing calls under one lock. Membership
order is a source-set enumeration, never an execution schedule.

## Independent scenarios

Selectors: interleaved(I)I, nested(II)I and locked(Ljava/lang/Object;I)I.
The native tick model records T(value), returns value + 1 with Java int wrapping,
and can throw RuntimeException on the supplied fault argument. No helper model
is an executed native implementation.

| Scenario | Required result and ordered behavior |
|---|---|
| interleaved(0) | 7, right initialization then right update; no helper. |
| interleaved(1), interleaved(-1) | 4, left initialization then left update. |
| nested(-1, 0) | -1; skips every update and call. |
| nested(0, 0), no fault | 4; T(0), T(1), T(2), T(3), each call once. |
| nested(1, 0), tick(1) faults | 4; T(0), T(1), handler adds one to the prior value, T(2), T(3). |
| nested(2, 0) | 4; two +2 updates, no helper. |
| nested(3, 2) | 4; two default +1 updates, no helper. |
| nested(0, 5) | 5; the first loop test exits, no helper. |
| locked(lock, 0), no fault | 3; acquire, T(0), T(1), T(2), release. |
| locked(lock, 0), tick(1) faults | Original RuntimeException escapes after acquire, T(0), T(1), release. |
| locked(null, 0) | NullPointerException before acquisition succeeds or any tick. |

Each original node has one leaf owner. A compound contains precisely its child
leaves, including handler and release portions. Every original operation retains
one origin. Shared switch keys do not duplicate a body, and original continuation
and lock destinations stay unchanged. The branch helper-free subset can execute
through the independent Rust selected-plan model. Loop/helper-fault paths require
LLM review; the model does not support loops or switches. VM execution is unrun.

## Preparation and observations

```sh
mkdir -p .cache/fixtures/member-forest
java -cp "$DIANA_SMALI_CLASSPATH" org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/member-forest/classes.dex quality/fixtures/composite/member-forest
cp .cache/fixtures/member-forest/classes.dex quality/fixtures/composite/member-forest/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/composite/member-forest/classes.dex \
  --exec '{"cmd":"p9-all","text_only":true}'
```

Prepared with Smali 2.5.2, API 24 and Temurin OpenJDK 21.0.12.1+1.
Expectations were recorded before candidate inspection. Against `02831ff`, P6
facts retain the same original member sets, entries, links and completion, and
P9 class text is identical. Three code members and one native declaration remain
retrievable without a stop or rendering error. Region member slices all borrow
the root leaf buffer; the former per-region vectors are removed. This is a
representation/lookup improvement, not a new natural recovery capability.

Thirty-one scoped nextest checks pass, including the three member-forest tests
and three executed branch scenarios. Deliberately dropping a leaf from the member
view fails the independent root-set assertion. The original source was restored.
This is checker sensitivity, not a passing mutation or VM behavior. The required
scope also covers fragment-order, operation-records, joint-expression-cover,
cast-carrier-walk, cast-preparation-order, coroutine-flows and suspend-loop-bodies.
The loop/switch paths have LLM review, not executed loop-model or VM evidence.
