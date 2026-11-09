# Fragment contraction order

Independently authored Java and Smali, licensed under the repository Apache-2.0
license. Original owner: `Lfixture/FragmentOrder;`. Smali fixes the DEX shape;
Java explains behavior and is not compiled during preparation.

The mechanism under refactoring is ordering original normal links after selected
owners have contracted them. A whole selected loop may hide its internal cycle;
an uncontracted cycle must not become an acyclic Sequence. Every original leaf
retains one selected owner. Baseline: `d5c665e`. These are compatibility guards
for structural replacement, not demonstrations of a newly recovered capability.

## Independent expectations

| Original selector | Scenarios and behavior | Required structure |
|---|---|---|
| `branchLoop(I)I` | flag 0 returns 7; flags 1 and -1 return 3. No calls, heap effects or exceptions occur. | Preserve the guard and one while loop, including its original update and continuation. |
| `irreducible(I)I` | flag 0 visits A(+1), B(+2), A(+1), returning 4. Flags 1 and -1 visit B(+2), A(+1), B(+2), returning 5. | Both cycle entries remain available. Preserve explicit transfers; do not clone a half-cycle or generate a dispatcher. |
| `selfCycle()V` | Every execution repeats indefinitely without a result or effect. | Preserve `while (true)` and terminate analysis independently of modeled execution. |

The irreducible method's Java explanation repeats the first increment only to
explain behavior. Its Smali has one original A occurrence and one B occurrence.
Diana must preserve those identities. The new
[loop/switch challenge](../../composite/fragment-order-loops/README.md) retains
the contraction mechanism with aliased switch targets inside a guarded loop.
VM execution and a final-plan behavioral executor for these controls are unrun.

## Preparation

Use an available Smali classpath in `DIANA_SMALI_CLASSPATH`:

```sh
mkdir -p .cache/fixtures/fragment-order
java -cp "$DIANA_SMALI_CLASSPATH" org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/fragment-order/classes.dex quality/fixtures/branches/fragment-order
cp .cache/fixtures/fragment-order/classes.dex quality/fixtures/branches/fragment-order/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/branches/fragment-order/classes.dex \
  --exec '{"cmd":"p9-all","text_only":true}'
```

Expectations above were recorded before inspecting candidate output.

Prepared with Smali 2.5.2, API 24 and Temurin OpenJDK 21.0.12.1+1. The original
zero-delta goto assembly was rejected by P1; the published selfCycle uses nop
followed by a nonzero backward branch. Both Java and the corrected Smali loop
indefinitely without an effect.

Fresh ordinary dbg_shell inspection on the retained `d5c665e` binary and this
refactor delivers all three methods without a stop or rendering failure. P6,
text and reported issues agree. The baseline omits literal operation anchors;
the current output retains all four original operation anchors in each integer
method and the nop anchor in selfCycle. Four focused nextest checks include
this input and its separate challenge. These checks exercise structure, ownership
and origin retention; they do not execute loop behavior or a VM.
