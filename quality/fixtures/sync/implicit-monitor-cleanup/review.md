# Fixture: implicit-monitor-cleanup

## Current verdict — natural monitor and catch binding

Read the authored Java/Smali, helper model and complete baseline73fc879/candidate
outputs for the same DEX. P1/P3/P4/P5 agree. The target obstruction improves:
counts has one natural synchronized body and no explicit compiler catch/release;
caught binds its exact exception in catch(all v2).

### Semantics

| Subject/path | Reasoning and judgment |
|---|---|
| counts normal | Enter this before the first field read; A.size precedes reading S and S.size. Sum2+3 is computed while held. The single normal boundary release precedes the unchanged outside return5. Agreement. |
| counts A/S faults or null queues | The original exception prevents subsequent field/call/sum work. The boundary releases this once and propagates that exact error. A null receiver fails before executing its modeled size body. No second explicit handler release remains executable. Agreement. |
| caught normal | read(0) returns7 without entering the handler. Agreement. |
| caught read failure | The same captured E is bound once before note(E). note success precedes return-1; note failure escapes as its new error. Binding moves only an effect-free capture without copy/Phi obligations. Agreement. |

The existing release and private cleanup operations/terminals retain source
products and zero-width anchors. A qualified catch binding anchors its original
capture operation/statement on the header; receiving copies keep their order.
The input itself contains no such copies. This changes selected syntax, not raw
method identities, source instructions or graph facts.

The independent Rust model actually executed five counts scenarios and three
caught scenarios through the final emission products, checking results, precise
field/call/lock order and error identity. Unknown execution forms reject checking.
It is a fixture helper model, not a JVM/device or external ArrayDeque execution.
LLM review is primary; those model runs and origin checks are separate support.

The fresh [boundary challenge](../monitor-surface-boundaries/review.md) adds nested
and reentrant locks, typed recovery, extra work, changed throws, split returns,
different locks and an escaping capture. No contradiction was found in the
recorded paths. A natural surface is selected only with the owning P6 guarantees;
other readable explicit fragments remain available.
