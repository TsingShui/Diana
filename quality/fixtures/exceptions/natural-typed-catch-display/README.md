# Natural typed catch display

Fresh independent challenge after the P7 display draft, before first inspection.
Parent: [exception-only completion](../exception-only-completion/README.md).
Repository license applies. Case.java explains Case.smali; no source or native
model is supplied as Engine declarations. Original owner Lfixture/ReadableCatch;.
Subjects: run(II)I, rethrow(ILjava/lang/Throwable;)I, foreign(I)I, multi(I)I.

## Independent scenarios

The trigger is an already natural single typed catch plus its original uncaught
propagation. Both are represented by ordinary lexical syntax, so repeated chain
markers are redundant. Added interactions: a loop, failure-only sum states and
receives, an unused normal call result before a parameter throw, shared-handler
intervals separated by an unprotected effect, and a two-handler compatibility
case outside this narrow display rule.

Native step records step(row,mode), then throws identity A (IllegalStateException)
when row==mode; otherwise returns10+row. recover records recover(error,row,mode)
then returns-100-row. outside records outside(mode) and returns. Each can instead
throw a selected identity at that call before result commit. F is another
IllegalStateException, B is IllegalArgumentException and E/K are otherwise
unmatched Throwable identities. Null throw produces modeled NPE N. Arithmetic
wraps as int. These are pre-inspection models, not executed implementations;
other runtime/linkage behavior is unknown.

| Scenario | Original result and ordered effects |
|---|---|
| run(3,-1) | step(0,-1), step(1,-1), step(2,-1), return33. |
| run(3,1) | step(0,1), step(1,1) throws A, recover(A,1,1), step(2,1), return-79. Failed step contributes no result. |
| run with nonpositive limit | return0, no callback. |
| step(1,1) throws unmatched E | E propagates after the first two step events; no recover or later update/call. |
| recover(A,1,1) throws K | K propagates after that event; no row2 call and no failed recover result commit. |
| rethrow(-1,F) | step(0,-1), recover(F,0,-1), return-100. The normal step result is unused, not its effect. |
| rethrow(0,F/null) | step(0,0) throws A, recover(A,0,0), return-100; parameter is never thrown. |
| rethrow(-1,E/null) | step(0,-1), then identical E/N propagates with no recover. |
| foreign(-1) | step0, outside(-1), step1, return21. |
| foreign(0) | step(0,0), recover(A,0,0), return-100; no outside/second step. |
| foreign(1) | step0, outside(1), step1 throws A, recover(A,1,1), return-101, not-91. |
| foreign outside throws F/E | propagate that failure without recover/second step; outside is unprotected even for matching F. |
| multi(-1) | step(0,-1), return10. |
| multi(0) | step(0,0), recover(A,0,0), return-100. |
| multi step throws B/E | B returns-2 without recover; E propagates. A recover failure B is outside its own catches and propagates. |

Run/rethrow must omit redundant inline exn/chain lines while retaining original
sites, both ordered routes, source snapshots, receives and one original anchor.
Foreign must keep its explicit protection and unprotected outside call. Multi
retains its original three routes; broad multi-catch suppression is not claimed
by this first narrow rule. No normal result, extra call, catch priority, snapshot
or capture is fabricated to reduce visible notation.

## Preparation

```sh
mkdir -p .cache/fixtures/natural-typed-catch-display
java -cp "$SMALI_CLASSPATH" org.jf.smali.Main assemble --api 24 \
  quality/fixtures/exceptions/natural-typed-catch-display/Case.smali \
  -o .cache/fixtures/natural-typed-catch-display/classes.dex
cp .cache/fixtures/natural-typed-catch-display/classes.dex \
  quality/fixtures/exceptions/natural-typed-catch-display/classes.dex
```

Baseline1d5f508870b2de2a0b3e55dc2dfd41b55b2a52cc. Actual preparation used
Smali2.5.2, OpenJDK21.0.12.1 and API24, not an exact-version admission pin.
Original subjects are class0: foreign/method2, multi/method3, rethrow/method4,
run/method5. On the same bytes the baseline fails the three plain typed-catch
display checks and passes the two refusal checks; the candidate passes all five.
`run`, `rethrow` and `multi` drop only the redundant chain line, inline markers and
`H` labels; `multi` became covered once the rule generalized to ordered printed
catches. `foreign` text is unchanged. Both isolated suites pass 173
checks. The116-fixture comparison keeps complete P2–P5 and original P6 facts,
with12 changed texts reviewed individually; the same APK retains full coverage.
[review.md](review.md) records each path and the execution limits.
