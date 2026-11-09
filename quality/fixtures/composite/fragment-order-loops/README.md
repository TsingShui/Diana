# Guarded loop with aliased switch targets

Independently authored Java and Smali under the repository Apache-2.0 license.
Original owner: `Lfixture/FragmentOrderLoops;`. This is a complexity challenge for
[fragment ordering](../../branches/fragment-order/README.md), against `d5c665e`.
The candidate shares a normal-link contraction kernel and one atomic ownership
installer. The challenge adds a guarded while loop, multiple repeated transfers,
and two original switch keys selecting one static body. It does not require a
method-shape handler or a new analysis route.

## Independent expectations

Selector: `nestedSwitch(II)I`. Arguments are `(flag, start)`.

| Scenarios | Result | Executed original updates |
|---|---|---|
| (0, -1), (0, 0), (0, 5) | 7 | None; the guard skips the loop. |
| (1, -1) | 4 | default +1, case 0 +1, case 1 +1, case 2 +2. |
| (1, 0), (-1, 0) | 4 | case 0 +1, case 1 +1, case 2 +2. |
| (1, 1) | 4 | case 1 +1, case 2 +2. |
| (1, 2) | 4 | case 2 +2. |
| (1, 3) | 4 | default +1. |
| (1, 4), (1, 5) | 4, 5 | None; the first loop test exits. |

No helper, heap effect, exception handler or lock occurs. Every switch path
repeats through the original loop predicate. Case keys 0 and 1 share one body;
its source operation executes on either selection without static duplication.
The loop and switch retain natural controls, and every original operation and
terminal retains its identity and source anchor. An update must not become a
for-header role. Partial-arm admission may contract the established loop but
must still see its original boundary and all outside entrances.

The integer paths above are independent expectations, not executed VM results.
VM execution and a final-plan behavioral executor for these controls are unrun.
Expectations were recorded before inspecting candidate output.

## Preparation

```sh
mkdir -p .cache/fixtures/fragment-order-loops
java -cp "$DIANA_SMALI_CLASSPATH" org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/fragment-order-loops/classes.dex quality/fixtures/composite/fragment-order-loops
cp .cache/fixtures/fragment-order-loops/classes.dex quality/fixtures/composite/fragment-order-loops/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/composite/fragment-order-loops/classes.dex \
  --exec '{"cmd":"p9-all","text_only":true}'
```

Prepared with Smali 2.5.2, API 24 and Temurin OpenJDK 21.0.12.1+1. Fresh baseline
and candidate dbg_shell inspection retain identical P6, text and issues, with
one expressed method and no stop/rendering failure. The current product retains
all six original operation anchors; the baseline retains four. The focused
nextest challenge passes ownership, natural loop/switch and shared-body checks.
