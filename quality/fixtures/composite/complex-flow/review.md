# Fixture: complex-flow

enum state machine with switch, loop, try/finally and ternary. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — iteration-tail review

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
