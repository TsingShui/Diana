# Nested and multi-clause catch display

Fresh independently authored challenge after the generalized display draft, before
its first inspection. Parents:
[natural typed catch display](../natural-typed-catch-display/README.md) and
[exception-only completion](../exception-only-completion/README.md). Repository
license applies. Case.java explains Case.smali; nothing here supplies declarations
to Engine. Original owner: `Lfixture/LayeredCatch;`.
Subjects: `layered(I)I`, `catchBody(I)I`, `aliased(I)I`.

## Added interactions

The parent rule covered one plain typed catch plus method-level propagation. This
challenge adds three interactions that the generalized rule must keep sound:

- two enclosing catch levels share the same exception type, so lexical nesting is
  the only statement of priority;
- a `finally` sits between an inner catch and an outer catch;
- a call inside a catch body is protected by a different, inner handler only;
- adjacent typed clauses share one handler body, where a single printed clause
  cannot state both original types by position.

## Independent expectations

Recorded before first inspection. Native `step(row,mode)` records `step(row,mode)`
then throws A (IllegalStateException) when `row == mode`, otherwise returns
`10+row`. `recover(error,row,mode)` records the call and returns `-100-row`.
`finish(mode)` records `finish(mode)`. Any call can instead throw a selected
identity before committing its result. B is IllegalArgumentException; E and K are
otherwise unmatched Throwable identities; throwing null gives the modeled NPE N.
These are pre-inspection models, not executed implementations.

| Scenario | Required result and ordered effects |
|---|---|
| layered(-1) | step(0,-1), finish(-1), return10. |
| layered(0) | step(0,0) throws A; recover(A,0,0) in the inner handler, finish(0), return-100. The outer handler does not run while the inner handler succeeds. |
| layered recover throws B | step(0,0), recover(A,0,0), finish(0), then B. The outer catch must not regain that failure even though B is also unmatched there; the outer clause handles only its own protected regions, and the pending `finally` rethrows outside it. |
| layered step throws B or E | step(0,-1), finish(-1), then identical B/E: the inner clause does not match, `finally` runs, and B is not caught by the inner `IllegalStateException` clause. |
| layered finish throws K | step and/or recover events, one finish, then K; no result commit and no repeat. |
| catchBody(-1) | step(0,-1), return10. |
| catchBody(0) | step(0,0), recover(A,0,0), step(1,0), return-91. |
| catchBody(-1) with step(1) unreachable | the second call runs only on the recovery path. |
| catchBody step(1) throws B | step(0,0), recover(A,0,0), step(1,0), return-2. |
| catchBody recover throws K | step(0,0), recover(A,0,0), then K; no second step, no -2. |
| catchBody step(1) throws A or E | that failure propagates; A is the type of the *outer* clause, which does not protect its own catch body. |
| aliased(-1) | step(0,-1), return10. |
| aliased(0) | step(0,0), recover(A,0,0), return-100. |
| aliased step throws B | step(0,-1), recover(B,0,-1), return-100; both original types reach the same body with the same row argument. |

`layered` and `catchBody` must display without a repeated chain line, inline
`[exn cN]` markers or handler labels, because enclosing visible catches/finally
state every original clause in original order. `aliased` may omit them only when
one printed union clause enumerates both original types at that single handler
position; a printed clause that named fewer types would keep the chain. No normal
result, call, catch priority, snapshot or capture may be fabricated to shorten the
text.

## Preparation and evidence

```sh
mkdir -p .cache/fixtures/nested-catch-display
java -cp "$SMALI_CLASSPATH" org.jf.smali.Main assemble --api 24 \
  quality/fixtures/exceptions/nested-catch-display/Case.smali \
  -o .cache/fixtures/nested-catch-display/classes.dex
cp .cache/fixtures/nested-catch-display/classes.dex \
  quality/fixtures/exceptions/nested-catch-display/classes.dex
```

Baseline0d3da9a2595da9a2fa89a28f7a899a50f46d2a13. Actual preparation used
Smali2.5.2, OpenJDK21.0.12.1 and API24, not an exact-version admission pin.
Original subjects are class0: aliased/method0, catchBody/method1, layered/method4.
On the same bytes the baseline fails four checks and passes five refusals/guards;
the candidate passes all nine. `layered` and `aliased` lose their redundant chain
text; `catchBody` was already clean. The expectation revision for the union clause
is recorded in [review.md](review.md). Both isolated suites pass 173 checks, and
the 116-fixture comparison keeps complete P2–P5 and original P6 facts.
