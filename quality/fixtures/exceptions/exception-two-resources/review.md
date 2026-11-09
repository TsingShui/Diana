# Fixture: exception-two-resources

## Current verdict — terminal guard follow-up

Read the changed terminal alternatives and remaining body. The selected
return/throw keeps its original evaluation, while other copies, calls, accesses
and results remain on the opposite path in their original order. Both-edge guard
orientation preserves the predicate's exact meaning; no data, check or effect is
dropped.

Compared the same prepared DEX on 29f1b82 and this commit. P7 requires a complete
linear arm ending in a printed return/throw; opaque scopes are not exit evidence.
The [fresh guard challenge](../../branches/terminal-guard-arms/review.md) covers
actual other-edge Phi copies, parent scope, recovering catches and cleanup.
Original P2 through complete P6 products agree; this is bounded LLM path reading,
not behavioral execution. Historical execution claims below were not rerun.


## Prior verdict — whole-operation follow-up

Read changed nested resource arithmetic: the pure total is computed before the
same closes even where it lies outside a smaller inner try. First/second
acquisition and reverse close order, captured return values, original fault
priority and suppression/propagation paths remain. Moving a nonthrowing scalar
expression across a catch delimiter does not move it across a close effect.

The [pure-tail challenge](../../exceptions/pure-tail-finally/review.md) and
[carrier challenge](../../atoms/refined-carriers/review.md) record the owning
producer/consumer repairs and blocked drafts. This is bounded current source
reading; historical execution claims below were not rerun and do not certify
this output. No behavioral executor, device run or Java recompilation ran.


Two-resource try-with-resources. `ExceptionTwoResources.java` is the authored
reference; `classes.dex` was prepared with `javac --release 8 -g` and
`d8 --min-api 24` through `quality/build.sh`.

- Provenance: authored for this repository; no third-party code or data.
- Regenerate from the sources; never edit the DEX by hand.
- Original subject: `Lfixture/ExceptionTwoResources;->sum(I)I`.

## Prior verdict — round 3

The current text was compared directly with the Java reference, including both
resource helper bodies, using the ordinary Engine implementation at `650dade`:

```sh
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/exception-two-resources/classes.dex \
  --exec '{"cmd":"p9-all","text_only":true}'
```

### Semantics

This is a manual source/output reading judgment, not an automated execution result.

| Method | Semantics |
|---|---|
| `sum` | Equivalent by manual path comparison: evaluation, reverse close order, primary exception and suppression order agree. |

### Path comparison

The throwable identities below are distinct, and `addSuppressed` is assumed to
return normally for those cases.

| Path | Reference and recovered behavior |
|---|---|
| Normal completion | Construct first, construct second, read first, read second, compute the result, close second, close first, return the saved result. `sum(5)` yields 8. |
| First construction fails | Propagate the construction failure without closing an unacquired resource. |
| Second construction fails | Close only first; a first-close failure is suppressed on the construction failure. |
| A read fails with E | Attempt second close, then first close; rethrow E. The second read is skipped if the first read fails. |
| A read fails with E and both closes fail with C2/C1 | Record `E.addSuppressed(C2)` before `E.addSuppressed(C1)`; rethrow E. |
| Normal second close fails with C2 | Close first and propagate C2; if first close also fails with C1, suppress C1 on C2. Second is not closed again. |
| Normal first close fails | Propagate that failure without reentering first cleanup. |

The result expression is evaluated before either close, even though the printed
return appears after cleanup. The catch capture is an explicit `move-exception`
assignment, not another effectful helper call. If `addSuppressed` itself throws,
that failure leaves its catch and reaches the enclosing protection, just as in
the expanded resource protocol; the recovered text adds no retry.

The concrete helper `close()` only sets `closed = true`, and neither read occurs
after a successful close. Thus close-failure cases above are manual reasoning
about the recovered exception protocol, not faults exercised by these helpers.
Automated execution of these paths has not been performed.

### Readability

Handlers now belong to nested try/catch scopes instead of appearing after a
method-level return. The body needs no `exception_chain`, inline exception marker
or goto. Remaining noise is explicit capture storage, generated local names and
expanded cleanup rather than a compact `try (resource...)` declaration.

### Ranked fixes

Improve local names and cleanup concision while retaining primary exception
identity, reverse close order and suppression order.

## Rounds

- R3: Direct manual path comparison finds the resource behavior consistent with
  the source. Automated behavioral execution remains separate and unrun.
- R2: Nested protection became readable; the semantic judgment was left pending.
- R1, 2026-11-08: Handlers appeared after return and a
  cleanup try appeared after throw; the negative mark described that reading
  obstruction, not an established execution mismatch.
