# Exception-only protected completion

Fresh independently authored challenge after the completion-interface draft,
before its first inspection. Parent discovery:
[constructor-argument-chains](../../atoms/constructor-argument-chains/README.md).
Repository license applies. Case.java explains the actual Case.smali rather than
supplying declarations to Engine. Java's foreign explanation repeats a pure
predicate; Smali has only the original branch and one shared throw target.

Owner: `Lfixture/ExceptionCompletion;`.
Subjects: `typed(ILjava/lang/Throwable;)I`, `nested(ILjava/lang/Throwable;)I`,
`foreign(ILjava/lang/Throwable;)I`.

## Mechanism and independent expectations

Recorded before candidate inspection. The guarded core consists of touch and an
original throw terminal; all its completions are exceptional. A catch entry is
not a normal follow. Typed adds both selected and uncaught original exception
outlets. Nested adds a proper outer protection interval, caught-result commit,
ordinary-finally replacement and handler failure. Foreign adds an unprotected
call arriving at the shared throw without the core's ordinary entry. No later
start, duplicate handler or generated dispatcher is permitted to hide it.

Native model: touch records touch(mode), outside records outside(mode), consume
records consume(T,mode) then returns700+mode with int wrapping, finish records
finish(mode). Each call can instead throw its selected identity before normal
commit. F/T are IllegalStateException identities; E/K are otherwise unmatched
Throwable identities. Throwing null produces modeled NullPointerException N,
which is not the selected catch type. These are independent models, not native
implementations or facts supplied to Engine. Other runtime/linkage behavior is
unknown.

| Scenario | Required result and ordered effects |
|---|---|
| typed(2,F), normal touch | touch2, consume(F,2), return702. The throw enters the catch, not a normal follow. |
| typed(2,E) | touch2, then identical E propagates; no consume. |
| typed(2,null) | touch2, then N propagates; no consume. |
| touch throws T | touch2, consume(T,2), return702; the parameter F/E/null is never thrown. |
| touch throws E | touch2, then identical E; no consume or parameter throw. |
| consume(F,2) throws K | touch2, consume(F,2), then K; the handler does not catch its own failure or use an absent result. |
| nested(2,F), ordinary calls | touch2, consume(F,2), finish2, return captured702. |
| nested(2,E) | touch2, finish2, then identical E; no consume. |
| nested touch throws T | touch2, consume(T,2), finish2, return702. |
| nested consume throws E | touch2, consume(F,2), finish2, then E. |
| nested finish throws K | one finish2; K replaces the pending result/throwable, with no retry/suppression. |
| nested null parameter | touch2, finish2, then N. |
| foreign(0,F) | touch0, consume(F,0), return700; no outside. |
| foreign(2,F), outside normal | outside2, consume(F,2), return702; no touch. |
| foreign outside throws T or E | outside2, then identical failure propagates; no consume, even when T matches the protected catch type. |
| foreign(2,null), outside normal | outside2, then N; no touch/consume. |

Require a natural typed catch with no fabricated normal continuation, unchanged
original clause priority, terminal operands, source snapshots and receiving
phases. An unmatched edge remains uncaught. Nested can retain an explicit outer
cleanup when correspondence is unavailable; it must close once after the selected
body/handler outcome. Foreign must not acquire the unprotected outside effect.
It is a refusal guard, not a positive recovery witness.

## Preparation and evidence

```sh
mkdir -p .cache/fixtures/exception-only-completion
java -cp "$SMALI_CLASSPATH" org.jf.smali.Main assemble --api 24 \
  quality/fixtures/exceptions/exception-only-completion/Case.smali \
  -o .cache/fixtures/exception-only-completion/classes.dex
cp .cache/fixtures/exception-only-completion/classes.dex \
  quality/fixtures/exceptions/exception-only-completion/classes.dex
```

Baseline ffdea3b9a98f6cdcc93a07da7df8487581de0779. Actual preparation used
Smali2.5.2, OpenJDK21.0.12.1 and API24, not an exact-version admission pin.
Original subjects are class0: foreign/method2, nested/method3, typed/method6.
The same prepared bytes give one passing refusal and three failed recovery
checks on baseline, then four passes on the candidate. Typed gains its natural
catch and nested gains the inner typed owner, allowing its existing outer cleanup
to remain natural rather than ProtectionBoundary. Foreign text is unchanged.
Both isolated suites pass 168 checks; format, package Clippy and release builds
pass. The 115-fixture comparison retains P2–P5/original P6 facts without retrieval/
render failures. [review.md](review.md) records each path and the execution limits.
