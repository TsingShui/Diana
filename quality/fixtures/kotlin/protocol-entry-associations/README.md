# Shared continuation entry associations

Original selector: `Lfixture/AssociatedBody;->invokeSuspend(Ljava/lang/Object;)Ljava/lang/Object;`.
The authored Java explains the logical body; the Smali defines its original ABI.
No external Kotlin declarations are supplied to the engine.

Discovery: [protocol-common-controls](../protocol-common-controls/README.md)
already establishes one SuspendLambda reentry. Its separate recognizer cannot
admit several positive states. This challenge retains its decision/parameter
entry and adds states 5 and 2, two calls, a captured `this.seed`, two saved fields,
a negative first-result exit and a shared marker return. The first comparison
also places the marker on the left of an equality test. Ordinary site/result
correspondence must handle these differences without another body recognizer.
The shared marker/capture joins prevent a complete natural DecisionPlan in the
baseline. The shared input producer consumes original comparison edges and the
existing exact integer-comparison query without requiring that display plan.

Independent expectations, recorded before candidate inspection:

- `prepare(seed)` records `P(seed)` and returns `seed + 7` using int arithmetic.
- Each `load(key, this)` records `L(key)`. The scenario supplies a Number result,
  a suspension marker or a fault. It does not modify captured fields.
- `record(reply, seed)` records `R(reply, seed)`; `observe(reply, seed, first)`
  records `O(reply, seed, first)`. They can fault after recording their call.
- Number results are Integer objects. Cast/intValue preserve their integer value.
  A null reply fails at intValue; an incompatible reply fails at the cast.
- Initial and resumed `throwOnFailure` stop on the supplied failure. Entry check
  precedes the seed read and Prepare; each resume check precedes its logical tail.
- Boxing returns Integer(-1). Invalid state constructs and throws the original
  IllegalStateException("invalid state") without invoking any modeled helper.

| Scenario | Required result and ordered helper effects |
|---|---|
| seed 3, replies 5 and 9, immediate | Return the second Integer(9); P(3), L(10), R(5,3), L(5), O(9,3,5). |
| seed 3, first reply -2 | Return Integer(-1); P(3), L(10), R(-2,3). No second call. |
| First call suspends; resume state 5 with reply 5 | Before marker: P(3), L(10), savedSeed=3, label=5. Resume restores seed and continues with R(5,3), L(5), O(9,3,5), returning Integer(9). |
| Second call suspends; resume state 2 with reply 9 | Before marker: P(3), L(10), R(5,3), L(5), savedSeed=3, savedReply=5, label=2. Resume restores both captures and observes O(9,3,5). |
| Both calls suspend in successive invocations | Each marker returns without running its remaining logical tail. The joined effects equal the immediate scenario. |
| Prepare faults | P(3), then failure; no Load. |
| First Load faults | P(3), L(10), then failure; no Record. |
| Record faults | P(3), L(10), R(5,3), then failure; no second Load. |
| Second Load faults | P(3), L(10), R(5,3), L(5), then failure; no Observe. |
| Observe faults | All five helper effects, then failure; no normal return. |
| Resume check faults | No helpers from that logical tail; stored captures remain original facts. |
| Null/incompatible first or second reply | Stop at the original cast/intValue; later Record/Observe does not run. |
| Invalid label | Original invalid-state construction/throw; no helper effects. |

Recovery expectation: one inferred source body retains both calls, captured
reads, the negative exit and the original default/error/resume attachments.
Every original operation and control leaf has one selected static owner. Shared
marker-return content is retained once; successful publication and edge copies
have one execution owner. P6/P7 products and rendering are inspection evidence;
coroutine/VM execution is not established by these expectations.

Negative selector: `Lfixture/AssociatedBadResume;->invokeSuspend(Ljava/lang/Object;)Ljava/lang/Object;`.
Its additional `record(savedSeed, savedSeed)` occurs only after the first resume
check. With seed 3 and first reply 5, that path emits R(3,3) before R(5,3);
immediate completion emits only R(5,3). A failed resume check prevents both calls.
This path cannot share the ordinary source body. The complete explicit member
must retain both loads, both record sites and its original resume/default routes.

Publication selector: `Lfixture/AssociatedPublications;->invokeSuspend(Ljava/lang/Object;)Ljava/lang/Object;`.
This captureless variation performs Load(5), reads the first Number's intValue,
then performs Load(firstReply) and returns the second reply. With replies 7 and
9 its helper effects are L(5), L(7), and the return is the second Integer(9).
Either or both sites may return the marker and resume under states 5 and 2.
Each result has one successful publication to its existing merge storage. Both
marker edges reach the same original return, which must stay owned once. A
first cast/null/intValue fault prevents the second call; a resumed failure
prevents the corresponding tail. No captured field or helper effect is invented.
`Lfixture/AssociatedSwitchPublications;->invokeSuspend(Ljava/lang/Object;)Ljava/lang/Object;`
has the same behavior and expectations with an original sparse switch. Dispatch
encoding and resume-input origin are independent: this switch still reads the
original parameter, without requiring a Frame result field or constructor.

Authored specifically for Diana; no reference implementation was copied.
Prepared with Smali 2.5.2, API 24 and Temurin JDK 21.0.12.1+1. These are observed
tools rather than fixture admission pins. Set DIANA_SMALI_CLASSPATH to a local
Smali assembler/dependency classpath and run:

```sh
java -cp "$DIANA_SMALI_CLASSPATH" org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/protocol-entry-associations/classes.dex \
  quality/fixtures/kotlin/protocol-entry-associations
cp .cache/fixtures/protocol-entry-associations/classes.dex \
  quality/fixtures/kotlin/protocol-entry-associations/classes.dex
```

The prepared DEX retains all four original selectors above, both dispatch
encodings, the two state/call pairs and shared marker return. Before revision:
`a5e8fa6`. Candidate: the shared-entry replacement recorded in the engine README.
Both inspections use this same prepared input through ordinary Engine/dbg_shell.
The three positive members gain source bodies; the negative member retains
identical full text. Three checks in `crates/diana-decomp/tests/protocol_entry_associations.rs` cover
all four members, publication ownership, capture/site associations and original
operation/leaf spans. They pass as part of the 47-test scoped nextest set.
Helper, coroutine and VM execution remains unrun; the scenarios above are
independent expectations and the path review is an LLM judgment.
