# A redundant check cannot own a producer evaluation

Project-authored Java/Smali under Apache-2.0. Original owner:
Lfixture/RedundantConsumer;. The discovery in
[protocol-owned-reads](../../kotlin/protocol-owned-reads/README.md) lost a source
Move's origin after a redundant frame check consumed its evaluation. This fixture
retains the same eliminated-consumer trigger and adds initialized construction before an alias Move, an
intervening helper and helper faults. Baseline: `f859adf`.

The input allocates and initializes a same-image Container, then copies its
reference through an alias Move before the check.
These established allocation/initialization facts prove that the check adds no
behavior. A native return descriptor alone did not reproduce this trigger.
P7 must normalize the redundant root before covering producers. A zero-width
check anchor cannot execute a producer or become its sole ownership site. No
fixture name establishes redundancy or governs the rule.

## Independent scenarios

Selectors: immediate()Lfixture/Container; and barrier()Lfixture/Container;.
The explicit model records A for allocation, M for constructor completion work
and K for after. Allocation returns object 1 or faults with OutOfMemoryError.
The constructor invokes made after the successful Object super constructor;
made records M and returns unit or faults with RuntimeException. After records K
and returns unit or faults with RuntimeException. Class initialization and Object
super success are assumptions. Models are not executed native or JVM code.

| Path | Expected result and ordered events |
|---|---|
| immediate, normal | Object 1, [A,M]. |
| immediate, allocation faults | OutOfMemoryError, [A]. |
| immediate, constructor faults | RuntimeException, [A,M]. |
| barrier, normal | Object 1, [A,M,K]. |
| barrier, constructor faults | RuntimeException, [A,M]; after does not run. |
| barrier, after faults | RuntimeException, [A,M,K]; no normal return. |

The alias Move retains its original identity even when its storage is coalesced.
Every acting helper executes once. The barrier prevents moving construction after the later helper.
Every original instruction retains its origin. The limited independent Rust
selected-plan model can execute these call/reference/return paths; VM execution
is unrun. Expectations were recorded before candidate inspection.

## Preparation

```sh
mkdir -p .cache/fixtures/redundant-consumer-cover
java -cp "$DIANA_SMALI_CLASSPATH" org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/redundant-consumer-cover/classes.dex quality/fixtures/atoms/redundant-consumer-cover
cp .cache/fixtures/redundant-consumer-cover/classes.dex quality/fixtures/atoms/redundant-consumer-cover/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/atoms/redundant-consumer-cover/classes.dex \
  --exec '{"cmd":"p9-all","text_only":true}'
```

Prepared with Smali 2.5.2, API 24 and Temurin OpenJDK 21.0.12.1+1.
Baseline ordinary Engine output returns an unassigned v1 and omits its original
Move anchor in both methods. The candidate returns the established constructed
reference and preserves the alias origin. Six allocation/constructor/barrier
scenarios pass in the limited selected-plan model. Restoring the old consumer
rule rejects both normal paths with MissingValue, a correspondence failure
rather than unimplemented model semantics. The source was restored.
Thirty-five scoped nextest checks pass, including this case and the protocol
construction challenge. VM/device execution and complete design acceptance are
unrun. Path reasoning is recorded in review.md.
