# Captureless nested retry and latch entries

Fresh, independently authored challenge after the R1 draft repair, before first
inspection of this input. Parent:
[handler-retry-rejoin](../handler-retry-rejoin/README.md). No upstream code is
copied; the repository license applies. Java explains Case.smali.

Original owner: `Lfixture/RetryNesting;`.
Subjects: `nested(III)I`, `terminal(II)I`, `foreign(II)I`.
The causal trigger remains a captureless exception entry shared with normal
completion. Added interactions: nested loops, an effectful update prelude,
exception-only sum snapshots and a handler which is itself a retreating latch.
The foreign prefix is a genuine refusal, not a renamed positive case.

## Independent models and scenarios

Recorded before the first candidate inspection. Native attempt records
`(attempt,row,column)`, then throws IllegalStateException A iff
`10*row+column == fault`; otherwise returns `1+row+column` with int wrapping.
advance records `(advance,row,column)` and returns normally unless a scenario
injects a specified failure there. No implementation is supplied to Engine.
Other runtime/linkage/allocation behavior is unmodeled; these are not executions.

| Scenario | Independent result and ordered events |
|---|---|
| nested(2,2,-999) | 8; attempt(0,0), advance(0,0), attempt(0,1), advance(0,1), attempt(1,0), advance(1,0), attempt(1,1), advance(1,1). |
| nested(2,2,0) | 7; same event sequence, first attempt throws and its contribution1 never commits. |
| nested(2,2,11) | 5; same sequence, last contribution3 never commits. |
| Zero/negative rows or columns | 0, no attempt/advance event. Outer row initialization/update may still occur for empty columns. |
| advance(0,0) throws A | A propagates after the first attempt and advance, even though it is the catch type; advance is outside the attempt interval. No increment or later event. |
| attempt(0,0) throws Error E | E propagates with no advance, inner/outer update or later call. |
| terminal(2,-999) | 3; attempt(0,0), attempt(0,1). Index increments before each attempt. |
| terminal(2,0) | 2; same two attempts; failed first result does not commit, increment still happened before it. The handler's goto must not restart that iteration. |
| terminal nonpositive limit | 0, no attempt. |
| foreign(2,-999) | 3; prefix attempt(-1,0) contributes0, then attempt(0,0)=1 and attempt(0,1)=2. |
| foreign(2,-10) | 2; prefix attempt(-1,0) throws, shared increment executes before first header and skips index0, then attempt(0,1)=2. |
| foreign(0,-10) | 0 after the prefix attempt throws and the original shared increment; no body call. |

Required recovery: nested retains two original while loops, terminal retains one
while with the original pre-call increment, and no capture is fabricated.
Shared normal and exceptional arrivals retain their actual copies/receives.
A normal arrival does not run exception-only sum restoration. Foreign's prefix
arrival must not be classified as an internally entered loop. Explicit catches,
chains or gotos remain permitted when needed; no for/dispatcher is introduced.

## Preparation and evidence

```sh
mkdir -p .cache/fixtures/captureless-retry-nesting
java -cp "$SMALI_CLASSPATH" org.jf.smali.Main assemble --api 24 \
  quality/fixtures/exceptions/captureless-retry-nesting/Case.smali \
  -o .cache/fixtures/captureless-retry-nesting/classes.dex
cp .cache/fixtures/captureless-retry-nesting/classes.dex \
  quality/fixtures/exceptions/captureless-retry-nesting/classes.dex
```

Baseline 73fc8793a91d766d021d117aa83ddfd644f7ac82. Actual assembly used Smali2.5.2,
OpenJDK21.0.12.1 and API24, not an exact-version admission pin. Class0 subjects:
foreign/method2, nested/method3, terminal/method4. Native advance/attempt are0/1.
The candidate recovers two nested loops and one retreating-latch loop; foreign's
text is unchanged. [review.md](review.md) records path reasoning and limitations.
The nine iteration checks are baseline-sensitive (two passes/seven failures),
then nine passes on the candidate. Full isolated runs pass 155 checks in each
configuration; no fixture/native behavior is executed.
