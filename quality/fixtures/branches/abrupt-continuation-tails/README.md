# Already-completed display tails

## Independent expectations

Authored Java/Smali, no third-party input. Seed: branch-switch-string's first
original integer dispatch still printed a planned goto after an already-emitted
case break. The initial repair consumes the delivered unconditional block end
when selecting a display continuation; it does not recognize a string protocol
or infer any external hash/equality implementation.

This fresh challenge adds a shared-default integer switch under finally cleanup,
a throwing selected read, a recovering catch and a break that completes its own
switch before further work. Original owner: `Lfixture/TailCompletion;`.
Subjects are `decorated(III)I`, `select(III)I`, `caught(I)I`, `breakScope(I)I`;
concrete helper bodies are `matches(II)Z`, `read(II)I` and `mark(I)V`. Java's implicit constructor is not a Smali subject.
Trace begins at zero and wraps as int. Standard exception construction succeeds;
no asynchronous/linkage/allocation fault or concurrent trace writer is modeled.

select's cases share one original default entry when their match test fails.
Read records 100+value before returning value or throwing ISE("read") in mode1.
Finally mark30 runs once before either propagation or mark40/return. Unknown key
and failed matches must not execute read, even in mode1.

| Subject and arguments | Expected exit | Ordered mark events |
|---|---|---|
| decorated(0,0,0) | return 10 | 80,110,90,91 |
| decorated(1,1,0) | return 20 | 81,120,90,91 |
| decorated(2,2,0) | return 30 | 82,130,90,91 |
| decorated(0,1,0) | return -1 | 80,99,90,91 |
| decorated(99,0,0) | return -1 | 99,90,91; no matches call |
| decorated(0,0,1) | throw original ISE("read") | 80,110,90; no mark91 |
| select(0,0,0) | return 10 | 10,110,30,40 |
| select(1,1,0) | return 20 | 11,120,30,40 |
| select(2,2,0) | return 30 | 12,130,30,40 |
| select(0,1,1) | return -1 | 10,30,40; no selected read |
| select(99,0,1) | return -1 | 30,40 |
| select(0,0,1) | throw original ISE("read") | 10,110,30; no mark40 |
| caught(0) | return 9 | 50,51,52 |
| caught(1) | return 7 | 50,52 |
| breakScope(0) | return 7 | 60,62 |
| breakScope(3) | return 7 | 61,62 |

The additional decorated subject records these expectations before its first
inspection: a concrete matches call records 80+candidate, then the first dispatch
selects an existing integer discriminator; the second dispatch selects 10/20/30
or -1. Its selected read is under finally mark90, followed by mark91 only normally.
This preserves a shared-default first dispatch independently of external String
behavior. The initial select comparison needed no tail omission, so select stays
as a protected-boundary guard rather than being claimed as positive coverage.

The display may omit only the planned action after an unconditional printed exit.
It must retain the original transfer, terminal, copies and source origins. A
conditional end alone does not establish that every path exits. Propagation
through a trailing Sequence cannot cross a selected break/continue target or an
opaque Try/Loop/Switch/Synchronized boundary. In particular, a body throw may
recover in caught; a case break finishes breakScope's switch but must not skip
mark62. Source continuations remain in inspection facts; only the completed
emission placement can omit the second action.

Expectations are recorded before this fixture's first candidate inspection. No
behavioral execution result is claimed.

## Preparation

```sh
mkdir -p .cache/fixtures/abrupt-continuation-tails
java -cp '.cache/evolution-demo/smali-tools/*' org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/abrupt-continuation-tails/classes.dex \
  quality/fixtures/branches/abrupt-continuation-tails
cp .cache/fixtures/abrupt-continuation-tails/classes.dex \
  quality/fixtures/branches/abrupt-continuation-tails/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/branches/abrupt-continuation-tails/classes.dex \
  --exec '{"cmd":"p7-all","details":true}' \
  --exec '{"cmd":"p9-all","text_only":true}'
```

Prepared with Smali 2.5.2, API 24 and OpenJDK 21.0.12.1. Baseline is the recorded
working tree based on `ebbc003` after split monitor outlets and before completed-
tail display selection. Both comparisons consume the same prepared DEX. The
initial protected select case needed no omission and remains a boundary guard;
the additional decorated case exercises the same completed-tail mechanism before
its later read/cleanup. See review.md for the direct reading judgment and unrun
behavioral execution.
