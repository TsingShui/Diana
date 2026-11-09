# Fixture: complex-flow

## Current verdict — protected private exit completion

Compared the same prepared DEX on7354242 and this candidate. In run([I)I,
MIN_VALUE still skips only advance/count and reaches the original index update.
DONE now breaks the enclosing while directly instead of jumping outside it.
That break skips advance/count/index update and reaches the same normal steps
addition. Exhaustion reaches it once as well. The catch still adds the saved
applied count and rethrows; cleanup field accesses remain explicit and are not
incorrectly qualified as finally. No update moves into a header.

Read Java against all changed paths and both complete method texts; other
members are unchanged. Original P2/P3/P4/P5, edges, snapshots and copy obligations
agree. The [fresh challenge](../../exceptions/protected-loop-exits/review.md)
covers the enclosing abrupt-handler proof and refusals. This is bounded LLM
review, not a behavior executor/device/Java run.

## Prior verdict — while/do-while architecture correction

Read the protected array loop and its outward planned goto: ordinary completion
advances once, while the original abrupt route bypasses that update and reaches
its outside target. Predicate array-length evaluation remains before each test.

Compared the same prepared DEX on 8ac6d95 and this commit. Dedicated for recovery
contradicted the approved architecture and is removed, not retained behind a
switch. Updates stay in ordinary body positions with original targets/copies;
explicit gotos can be necessary instead of update-skipping while continues.
The [fresh challenge](../../loops/while-tail-continuations/review.md) covers
three-level private continuations, real shared entrances, cleanup and do-while.
This is bounded LLM source/path reading, not behavioral execution. Historical
execution claims below were not rerun and do not certify this output.


## Prior verdict — whole-operation follow-up

Compared the same prepared DEX with completed-eleven-repair bda9a49. Only
derived label/control coordinates change. Development-only correspondence found
identical printed words, original L/H program points and identical printed
lexical-scope bindings across every method publication; declaration-only members
are exact. Original P2/P3/P4/P5 facts agree. The unchanged semantic path review
below is reused, not replaced by a naming score or a claim of behavioral
execution.

The [pure-tail challenge](../../exceptions/pure-tail-finally/review.md) and
[carrier challenge](../../atoms/refined-carriers/review.md) record the owning
producer/consumer repairs and blocked drafts. This is bounded current source
reading; historical execution claims below were not rerun and do not certify
this output. No behavioral executor, device run or Java recompilation ran.


enum state machine with switch, loop, try/finally and ternary. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Prior verdict — iteration-tail review

The changed subject is Lfixture/ComplexFlow;->run([I)I. Java and current text were
reread. MIN_VALUE now skips the body without a goto but still performs the
original index update. DONE's retained goto L8 exits the for before that update
and reaches the same normal cleanup. advance still succeeds before applied++;
failure therefore leaves the current element uncounted. Both cleanup paths add
that same applied count to steps once. Other members are unchanged.

This is direct LLM source/path evidence on the candidate based on `ebbc003`, not
executed behavioral PASS. Original P2/P3/P4/P5 and relevant P6 control/exception/
lock facts agree in corpus inspection. Unchanged member text reuses prior reviews.
The new [iteration-tail challenge](../../loops/iteration-tail-cleanup/review.md)
records the interacting cases and unsupported execution scope.

## Prior verdict — round 2

LLM reread of the Java reference, original delivered control/storage facts and
candidate text based on `ebbc003`. The changed method is
`Lfixture/ComplexFlow;->run([I)I`; other member text is unchanged and prior
reviews are reused. No scenario execution was performed.

The recovered try now includes the nonthrowing applied/index initialization,
without admitting an unguarded fallible operation. Each MIN_VALUE element still
jumps to the index update without advancing or counting. DONE jumps out of the
loop but remains inside the protected body until its normal cleanup. Other
elements call advance before incrementing applied; a throwing advance therefore
does not count that element. Both normal and exceptional paths add the current
applied count to steps once, and the handler rethrows the captured object.
A failure in cleanup escapes without a retry. Null input records applied=0 before
the first length read can fail. Int additions retain original wrapping behavior.

The catch-all is more readable, but exit gotos and separate normal cleanup remain.
This is a source/path judgment, not a new executed equivalence result. Original
facts and member availability were preserved in the corpus comparison.

## Prior verdict — round 1, 2026-11-08

### Semantics

The verdict is written by reading the reference next to the output.

| method | semantics |
|---|---|
| `advance` | ✔ Inverted ternary/nested If selections agree; DONE/FAILED correctly take default. |
| `run` | ✔ Continue/break use gotos; finally writes steps on normal and exceptional paths. |
| `summarize` | ✔ The conditional result expands into locals and If with equivalent selection. |

### Findings

- default: break is printed first; case 0/1 order remains. Recorded forms: `switch`, `default: break;`.
- The RUNNING/FAILED ternary expands into If/else assignments to a local. Recorded forms: `state = input > 0 ? RUNNING : FAILED`.
- The loop contains an empty If and goto L6/L8/L9/L10/L12. Recorded forms: `if (state != DONE) { }`.
- history[steps] reads this.steps three times; the recorded path had no intervening side effect, but the presentation is fragile. Recorded forms: `history[steps] = input`.

### Ranked fixes

Improve enum/ternary presentation, empty branches and gotos; avoid redundant field-read presentation. Recorded forms: `this.steps`.

## Rounds

- R1 2026-11-08: default: break is printed first; case 0/1 order remains. Recorded forms: `switch`, `default: break;`.
