# Shared iteration tails through cleanup

## Independent expectations

Authored Java/Smali, no third-party input. Discovery:
`../../exceptions/typed-catch-loop-exits/`. The initial repair recovers that
method's original pure row update as a for iteration, instead of treating its
update-skipping value-2 branch as an ordinary while continue. This challenge
adds nested loops, local continuations, typed shared handlers, per-iteration
finally, early return/break, and two deliberate update-admission barriers.

Owner: `Lfixture/IterationTail;`. Subjects:
`nested(III)I`, `skipUpdate(II)I`, `locked(Ljava/lang/Object;I)I`.
All helper bodies are in the DEX. trace starts at zero. Standard constructors
succeed; no asynchronous/linkage/allocation fault or concurrent trace write is
modeled. Java's implicit constructor is not a Smali subject.

read marks 10+code before its mode-selected result/failure. NFE/NPE recover 100
after mark 20; RuntimeException recovers -2 after mark 21. cleanup marks 30+code,
then modes 5/6 throw IAE("close"). Other modes match the parent helper model.

| Subject | Arguments | Expected exit | Ordered effects |
|---|---|---|---|
| nested | (2,0,0) | return 13 | For codes 0..4: read mark then cleanup mark; value 2 skips addition, value 5 breaks only the inner loop |
| nested | (2,0,1) | return 4 | Codes 0..2 only; row 1 skips the inner loop but still performs its outer update |
| nested | (2,1,0) or (2,2,0) | return 600 | Codes 0..5: 10+code,20,30+code |
| nested | (2,1,1) | return 300 | Codes 0..2: 10+code,20,30+code |
| nested | (2,3,0) | return -2 | 10,21,30; neither update runs after return |
| nested | (2,4,0) | throw original Error | 10,30 |
| nested | (2,5,0) | throw IAE from normal cleanup | 10,30; no addition/update |
| nested | (2,6,0) | throw IAE replacing Error | 10,30 |
| nested | (0,6,1) | return 0 | none |
| skipUpdate | (3,0) | return 0 | 50,51,52 |
| skipUpdate | (3,1) | return 2 | 50,50; direct header continuation deliberately leaves row=0 |
| locked | (lock,2) | return 2 | LOCK,mark(0),UNLOCK,LOCK,mark(1),UNLOCK |
| locked | (null,1) | throw NPE at acquisition | no mark/release |
| locked | (null,0) | return 0 | none |

nested must preserve both original update points. The pure inner update is owned
inside the enclosing loop but not by a sibling or nested private-exit corridor.
A return, break or failed cleanup must not accidentally execute an update.

skipUpdate has two original backedges, only one of which performs row++. A for
that automatically increments on its direct header continuation would change the
second mark from 50 to 51. locked has specialized release-to-step routing; until
that producer supplies a compatible source continuation, its update stays visible
in the ordinary while rather than hiding a still-targeted label in a header.

These expectations were written before candidate inspection. No scenario executor
or Java-recompilation result is claimed.

## Nonconstant update discovery

Added stride(II)I independently before inspecting its first output. The shared
pure row update reads the original step parameter, not a literal. A source for
schedule may retain that exact read at the iteration point; it must not assume
step is positive, constant or loop-invariant in general. No pre-test assignment
moves into the header. Scenarios: (0,0) returns0/no events; (7,2) returns12 with
marks0,2,4,6; (5,3) returns3 with marks0,3; (1,2147483647) returns0 with mark0.
Zero/negative-step nontermination for positive limits is not a termination claim.
The unchanged mark helper's wrapping trace arithmetic applies. The same prepared
DEX is compared before/after the nonconstant-read admission repair; the other
methods retain their original Java/Smali instructions and expectations.

## Preparation

```sh
mkdir -p .cache/fixtures/iteration-tail-cleanup
java -cp '.cache/evolution-demo/smali-tools/*' org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/iteration-tail-cleanup/classes.dex \
  quality/fixtures/loops/iteration-tail-cleanup
cp .cache/fixtures/iteration-tail-cleanup/classes.dex \
  quality/fixtures/loops/iteration-tail-cleanup/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/loops/iteration-tail-cleanup/classes.dex \
  --exec '{"cmd":"p6-all","details":true}' \
  --exec '{"cmd":"p9-all","text_only":true}'
```

Prepared with Smali 2.5.2, API 24 and OpenJDK 21.0.12.1. Baseline: recorded
working tree on `ebbc003` after the first three repairs and before iteration-tail
recovery. Both comparisons consume the same prepared DEX. The initial challenge
recovered only the outer update; corrected ancestor ownership now retains both.
See review.md for the direct semantic judgment and unrun execution boundary.
