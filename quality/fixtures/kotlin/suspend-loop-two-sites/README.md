# Two suspend sites inside one loop

This project-authored input extends the
[closed total loop](../kotlin-suspend-state/README.md) with two suspension sites,
an effect between them, and an early loop exit after the first result. It has
two distinct interior reentries and saves limit, sum, index, and the first result.
Sources use the repository license. Java/Kotlin explain the logical run; Smali
fixes the ABI and is the prepared-input authority. The subject is
`Lfixture/SuspendTwoSites;->total(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;`.

## Independent helper model and scenarios

`load(a)` records `load(a)` and normally yields boxed `2 * a`, immediately or
after suspension. `record(v)` records its value. Helpers can fail at a selected
call. The input frame callback stores the result/failure, sets the high bit,
and calls total with zero limit; both resume paths restore the real limit.
Valid frames and successful frame-field/type resolution are model assumptions.
No helper/runtime executor is implemented.

| Scenario | Expected result and effect order |
|---|---|
| limit 0 or -2 | Return 0; no helpers. |
| limit 1, any immediate/resumed combination | `load(0), record(0), load(1)`; return 2. |
| limit 2, any immediate/resumed combination | `load(0), record(0), load(1), load(1), record(2), load(3)`; return 10. |
| limit 3, alternating suspension sites | The previous sequence, then `load(2), record(4), load(5)`; return 24. |
| Third load call (argument 1) returns -3, limit 3 | First iteration contributes 2. Then `load(1), record(-3)` exits; return 2; no second load for that iteration. |
| Second load call (argument 1) fails | `load(0), record(0), load(1)` throws; no sum update or increment. |
| Third load call fails, limit 3 | The first iteration completes; its record remains. The next first load throws before record or another load. |
| Record fails | The first result has been obtained; second load, sum update, and increment are skipped. |
| Either site returns marker | No conversion or later work at that site runs until completion; the other site's tail is not selected. |
| Invalid adjusted label | Throw the original invalid-state error before loop work. |

The two calls with argument 1 in the limit-2 scenario are different occurrences:
the first is iteration zero's second site, and the next is iteration one's first
site. Fault selection uses call ordinal plus argument, not argument alone.
State one restores limit/sum/index; state two also restores the first result.
A future source recovery MUST retain the record and break before the second
call, and update sum before index only after successful second completion.

## Preparation

```sh
java -cp "$SMALI_CLASSPATH" org.jf.smali.Main assemble --api 24 \
  quality/fixtures/kotlin/suspend-loop-two-sites \
  -o .cache/fixtures/suspend-loop-two-sites/classes.dex
cp .cache/fixtures/suspend-loop-two-sites/classes.dex \
  quality/fixtures/kotlin/suspend-loop-two-sites/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/kotlin/suspend-loop-two-sites/classes.dex \
  --exec '{"cmd":"classes"}' --exec '{"cmd":"p9-all","text_only":true}'
```

Expectations preceded candidate-output inspection.

## Observed validation

Preparation used Smali 2.5.2, OpenJDK 21.0.12.1, API 24 and DEX 037, without an
admission pin. The `b14cd40` baseline delivered both classes and all five methods
but kept the label `switch` and an irreducible graph around both resume entries.
The candidate keeps the same members and renders
`public static suspend Object total(...)`, one `while` over the original head
predicate with its exit copy, the record, and the negative `goto` to the shared
boxed return. Both interior resume corridors, state publication and marker
returns remain ABI attachments. The Engine test checks the delivered cycle
partition and the source shape; the review records the path-by-path comparison.
Fixture revision `69d3b74` adds no production route. The four new fixture checks
and complete 103-test engine nextest run passed. The 97-fixture dev comparison
against `dd70d20` preserves every text/issue list and member. Workspace format,
all-target/all-feature Clippy and dev build passed. The review is LLM path
reading under the helper model. No release or helper/scheduler/final-plan
behavioral execution was run.
