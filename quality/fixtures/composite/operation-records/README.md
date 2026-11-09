# Operation-owned bodies, snapshots and lock outlets

Independently authored Java/Smali under the repository Apache-2.0 license.
Original owners are `Lfixture/OperationRecords;` and `Lfixture/RecordBox;`.
Smali fixes the input. This challenge extends
[joint expression ownership](../../atoms/joint-expression-cover/README.md) with
protected initialization, a saved-value observer, constructor delegation and
normal/exceptional lock completion. Baseline: `76f19c8`.

The refactor stores body data in original operation records, removes the separate
statement arena/reverse map and consumes one selected operation phase schedule.
Construction body ownership must not relocate allocation, initialization,
argument preparation or saved-state events. Names alone establish none of these
relations. No helper implementation or external declaration is imported.

## Independent behavior and recovery expectations

Original selectors are `protectedNew(I)I`, `locked(Ljava/lang/Object;I)I`,
`RecordBox.<init>(II)V`, and native declarations first(I)I, later()I, gate(II)V.

The explicit helper model records F(seed), returns seed + 10 with Java int
wrapping, and may throw R from first. Later records L, returns 20 and may throw R.
Gate records G(a,b) and may throw R. R is a RuntimeException object. Object's
super constructor and established class initialization add no modeled event or
fault. Allocation may fail with OutOfMemoryError before later; it is not caught
by this RuntimeException clause. Models are expectations, not executed helpers.

| Path | Result/completion | Ordered helper/lock events |
|---|---|---|
| protectedNew(0), normal | 11 | F(0), allocation, L, G(10,20). |
| protectedNew(1), normal | 12 | F(1), allocation, L, G(11,20). |
| first faults | 0 from the handler | F(seed); allocation/later/gate do not run. |
| later faults after first returns 10 | 10 from the handler | F(0), allocation, L; gate does not run. |
| gate faults after first returns 10 | 10 from the handler | F(0), allocation, L, G(10,20). |
| allocation fails | Original OutOfMemoryError escapes | F(seed), failed allocation; later/gate do not run. |
| locked(null, seed) | NullPointerException before first | Failed acquire; no release/helper event. |
| locked(lock, -11), normal | -1 | acquire, F(-11), release; later does not run. |
| locked(lock, 0), normal | 20 | acquire, F(0), L, release. |
| first or later faults under lock | Original R escapes after release | acquire, reached helpers, release, throw R. |

The protected saved value commits before subsequent faults; before first faults
the handler observes zero. A failed initializer never performs the success-only
increment. Each call/allocation keeps its original instruction origin once.
The RecordBox constructor keeps super delegation before gate. Both native and
code members remain retrievable. The lock scope releases exactly once on every
acquired exit and does not synthesize another release at its closing brace.

The source/output review inspects copies, protection, initialization and lock
completion through the ordinary Engine path. Rust's independent selected-plan
subset executes the listed constructor/lock helper scenarios; it rejects
unmodeled control and acting snapshot operands. Native helpers, Object's
constructor, allocation and locks use the explicit model above, not a JVM.
VM/device execution remains unrun. Expectations were recorded before candidate
inspection.

## Preparation

```sh
mkdir -p .cache/fixtures/operation-records
java -cp "$DIANA_SMALI_CLASSPATH" org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/operation-records/classes.dex quality/fixtures/composite/operation-records
cp .cache/fixtures/operation-records/classes.dex quality/fixtures/composite/operation-records/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/composite/operation-records/classes.dex \
  --exec '{"cmd":"p9-all","text_only":true}'
```

Prepared with Smali 2.5.2, API 24 and Temurin OpenJDK 21.0.12.1+1. Baseline and
candidate dbg_shell inspection deliver three code and three native declarations
with no stop/rendering failure. The baseline misses the common save before New;
the handler returns zero after later/gate faults although saved has committed.
The candidate emits the selected save before the New evaluation. Three focused
nextest checks pass, including eleven modeled constructor/lock scenarios.
Restoring the old phase owner fails the later-fault scenario with 0 instead of 10.
This sensitivity run is an intentionally failed mutation, not a passing gate.
