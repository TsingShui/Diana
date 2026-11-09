# Scalar views through protected joins

## Parent and added interaction

Independently authored after the draft repair of
[scalar-use-views](../../atoms/scalar-use-views/README.md), before this challenge's
first candidate inspection. No application implementation is copied.
The repository license applies. Java explains the Smali assembly input.

Original owner: `Lfixture/ProtectedViews;`.
Subjects: `route([CIZ)I`, `composed(I)I`, `raw([CZ)I`, `unproved(I)I`.
The trigger remains a range-proved char use of a word carrier, followed by an
int use of the same value. New dimensions are a loop, two producer kinds joining
through executable Phi copies, typed recovery, finally cleanup, partial result
commit and a composed producer. `raw` is a compatibility guard, not a repair
witness: a numeric-only use of a char-valued Phi stays numeric.

## Independent scenarios and helper model

Recorded before first inspection. `unit` has an actual mark/add/int-to-char body;
`record(C)` and `record(I)` have actual mark/add bodies. `mark(code)` is the only
native helper: record code in an ordered trace, then throw the throwable assigned
to that code in a scenario's fault map, otherwise return normally. Faults are
identified by throwable identity, not merely class; unlisted runtime/allocation
failures are unmodeled. Source arithmetic wraps; unit keeps the low UTF-16 unit
at its original int-to-char instruction. No view adds another conversion.

| Scenario | Result/throwable and ordered trace |
|---|---|
| `route({'A','B'},2,true)`, no faults | `6262`; `[1065,2065,70,1066,2066,71]`. Each char result commits before the int call; each cleanup precedes the row increment. |
| `route(null,2,false)`, no faults | `6262`; `[10,1065,2065,70,11,1066,2066,71]`. No array read occurs on this branch. |
| Nonpositive limit | `0`, empty trace; no producer, record or cleanup executes. |
| Fault1065 = IllegalStateException A, otherwise normal | `3132`; `[1065,40,70,1066,2066,71]`. First row commits neither record result. |
| Fault2065 = IllegalStateException A | `4197`; `[1065,2065,40,70,1066,2066,71]`. First row retains the char result1065; int result does not commit. |
| Fault1065=A and fault40=B | Throw B after `[1065,40,70]`; cleanup runs on recovery failure, no later row. |
| Fault1065=A, fault40=B and fault70=C | Throw C after `[1065,40,70]`; ordinary finally replaces B and does not run twice. |
| Fault70=C on normal first row | Throw C after `[1065,2065,70]`, no second row despite total3130 already computed. |
| Fault1065 is Error E, not IllegalStateException | Throw E after `[1065,70]`; no recovery mark40. |
| From-call fault10=A | Throw A after `[10]`, before the try; no record or cleanup. |
| From-array null/empty data, positive limit | Null/bounds failure before try; no mark or cleanup. |
| `composed(1)`, no faults | `1066`, trace `[11,1066]`; unit executes once before record, whether composed or separate. |
| `composed(1)`, fault11=A | Throw A after `[11]`, no record mark1066. |
| `raw({'A','B'},true/false)` | `65` / `66`; no mark, no added runtime conversion or char view on the numeric return/copies. |

Required recovery: the selected char argument states its view, while the int
argument and raw word transfers do not inherit it. Preserve original calls,
copy/snapshot phases, handler priority, pending results/throws and type-use/read
spans. Explicit protection is allowed; a new natural finally is not an output gate.
These are independent models, not executed behavior results.

The additional guard `unproved(I)I` is recorded before its first inspection.
It passes an unconstrained int parameter directly to the original char and int
bindings. There is no int-to-char instruction. P4 cannot establish a lossless
char view from the full int range, so the renderer retains the raw word and
original char signature disclosure rather than inventing a view or truncation.
For input65, the modeled calls give3130 and trace `[1065,2065]`. Out-of-range
argument ABI behavior is unknown, not claimed equivalent to an invented Java
cast. Case.java documents this DEX-only difference instead of adding a fake
conversion. This is a preservation guard, not a recovery-benefit witness.

## Preparation and status

```sh
mkdir -p .cache/fixtures/scalar-views-protected-joins
java -cp "$SMALI_CLASSPATH" org.jf.smali.Main assemble --api 24 \
  quality/fixtures/exceptions/scalar-views-protected-joins/Case.smali \
  -o .cache/fixtures/scalar-views-protected-joins/classes.dex
cp .cache/fixtures/scalar-views-protected-joins/classes.dex \
  quality/fixtures/exceptions/scalar-views-protected-joins/classes.dex
```

Baseline: `df10b744985e3407f902442b8252fab100700aec`.
The candidate is this scalar-use rendering change. Observed assembly used
Smali2.5.2, OpenJDK21.0.12.1 and API24, not a per-fixture admission pin.
Class0 subjects: composed/method0, raw/2, route/5, unit/6 and unproved/7.
The final prepared bytes were used unchanged on baseline and candidate.

```sh
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/scalar-views-protected-joins/classes.dex \
  --exec '{"cmd":"p9-all","text_only":true}'
cargo nextest run --locked -p diana-decomp --test scalar_use_views --no-fail-fast
```

The protected-view and composition checks fail before the repair and pass after
it; raw and unproved guards pass on both. Across all four linked fixtures, the
final baseline check is six passes/seven failures versus13/13 on the candidate.
A preliminary raw guard wrongly required executable copies for a coalesced Phi;
it now checks both original array reads and the numeric result, while route
still requires executable join/snapshot copies. No independent expected result
was changed. [review.md](review.md) records path reasoning and evidence limits.
These are product/text checks; the native helper model was not executed.
