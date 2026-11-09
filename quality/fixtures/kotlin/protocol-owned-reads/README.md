# Owned reads across construction and coroutine completion

Project-authored Apache-2.0 fixture, derived from the normal paths in
[suspend-loop-captured-effects](../suspend-loop-captured-effects/README.md) and
[suspend-three-step-effects](../suspend-three-step-effects/README.md).
The new challenge adds a grouped ReadBox construction and observable use after
normal/resumed completion, before the next loop update or early-exit test.
Baseline: `f859adf`. Original owners are Lfixture/OwnedLoop;, Lfixture/OwnedFlow;
and Lfixture/ReadBox;, with their original Frame declarations.

The shared visibility query consumes P7 operation bodies, original read origins,
construction arguments, terminals and selected edge copies. It removes duplicated
upstream-use scans without another SSA/region solver. Body ownership, selected
execution and saved storage remain distinct. A construction body has no ordinary
input list; its delivered argument reads still need their actual source storage.

## Independent scenarios

Selectors: total(IILkotlin/coroutines/Continuation;)Ljava/lang/Object; and
walk(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;.
Java explains the normal source behavior; Smali fixes the initial/resumed ABI.
All integer arithmetic wraps as Java int. Prepare records P(i) and returns i + 10.
Record records R(value). Observe reads the supplied box fields and records O(a,b).
Box allocation records A; its constructor stores first then second after a
successful Object super constructor. No model is an executed native/JVM helper.

Loop load records L(argument) and normally returns that int through Number.
It may complete immediately, return the suspended marker and later resume with
that same result, or deliver a fault. Flow loads use the scenario's supplied
integer replies. Frame and marker are distinct references. An error resumed
through throwOnFailure propagates before downstream work. Invalid states throw
IllegalStateException. Successful class initialization and Object/Frame super
construction are explicit environmental assumptions; general runtime faults
are not inferred away by the engine.

| Scenario | Required observation |
|---|---|
| total(0,2), initial | 2; no prepare/load/box/observe/record. |
| total(1,2), normal or resumed load | 14; P(0), R(2), L(10), A, O(12,2), R(12). |
| total(2,2), either load resumes | 28; the first trace above, then P(1), R(2), L(11), A, O(25,2), R(25). |
| second loop load faults | Prior first trace, then P(1), R(2), L(11), fault; no second box, observe or final record. |
| first box allocation faults | P(0), R(2), L(10), failed A; no O or later checksum/update/record. |
| walk(8), replies 1,2,3 | 6; L(8), R(1), L(2), R(2), A, O(1,2), L(3), R(3). |
| first flow reply -2 | -1; L(seed), R(-2); no box or later load. |
| replies 1,0 | 1; L(seed), R(1), L(2), R(0), A, O(1,0); no third load. |
| third flow load faults | Two reply records and the box/observe happen before L(3), fault; no third record. |
| box allocation faults with second reply zero | Fault after R(0), failed A; the early return does not run. |
| resume flow state 2 | Restore first and consume the second reply; box/observe run once after second success. |
| resume flow state 3 | Restore first and second; box/observe already happened and do not execute again. |

The resulting loop, branch choices, read identities, exact continuation occurrence,
constructor argument order, copies and member availability must be retained.
Unknown acting operations or operand facts remain gaps, not fabricated values.
Expectations precede candidate inspection. Coroutine/VM execution is unrun; path
review and actual product checks provide distinct evidence.

## Preparation

```sh
mkdir -p .cache/fixtures/protocol-owned-reads
java -cp "$DIANA_SMALI_CLASSPATH" org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/protocol-owned-reads/classes.dex quality/fixtures/kotlin/protocol-owned-reads
cp .cache/fixtures/protocol-owned-reads/classes.dex quality/fixtures/kotlin/protocol-owned-reads/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/kotlin/protocol-owned-reads/classes.dex \
  --exec '{"cmd":"p9-all","text_only":true}'
```

Prepared with Smali 2.5.2, API 24 and Temurin OpenJDK 21.0.12.1+1.
Both revisions retain six code and seven no-code members without a stop or render
failure. The shared observer query alone preserves text on this input and its
three parent/cycle comparison groups. Removing the redundant consumer also
recovers the previously missing frame-Move origin and its available source code.
The two new protocol tests and the 35-test scoped nextest set pass. They check
constructor use order, actual selected storage and the implicit continuation;
existing forwarding remains part of that selected product.
The independent alias/constructor model exposes the omitted-consumer defect in
atoms/redundant-consumer-cover. These coroutine paths have LLM path review but
no executed coroutine/VM result. Full protocol/IR replacement remains incomplete.
