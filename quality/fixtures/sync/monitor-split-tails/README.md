# Split monitor outlets under an outer capture

## Independent expectations

Authored Java/Smali, no third-party input. Seed:
`../monitor-iteration-exits/` walk, whose closed scope has distinct unlocked
continuations. The initial repair keeps every original outlet explicit when
there is no shared normal follow, rather than rejecting the whole natural scope.
It preserves held closure, capture proof, original targets and post-release copy
restrictions. This fresh challenge adds nested independent captures, three inner
release/tail targets, shared typed recovery, held cleanup and failures in unlocked
inner tails that remain held by the outer capture.

Owner: `Lfixture/MonitorSplit;`. Subject:
`split(Ljava/lang/Object;Ljava/lang/Object;II)I` and
`transfer(Ljava/lang/Object;II)I`.
All helper bodies are in the DEX; Java's implicit constructor is not a Smali
subject. Trace starts at zero and mark uses wrapping int arithmetic. Standard
exception construction succeeds. No asynchronous/linkage/allocation fault or
concurrent trace writer is modeled.

The Java explanation selects its tail after inner release; Smali uses the same
pure comparisons of the unchanged route parameter immediately before release to
give the three tails distinct original outlets. This does not move a call or
observable read. Route values other than 0 and 1 select the third tail and call
finish with argument 2, not the original route value.

Notation: LO/LI=outer/inner acquisition; UO/UI=matching releases; R=mark10;
C=mark30; T(k)=mark(40+k). Cleanup runs while both obligations are held.
Tail helpers run AFTER UI and BEFORE UO. Returning a tail value releases UO first.

| Arguments (outer,inner,route,mode) | Expected exit | Ordered effects |
|---|---|---|
| (O,I,0,0) | return 6 | LO,LI,R,C,UI,T(0),UO |
| (O,I,1,0) | return 10 | LO,LI,R,C,UI,T(1),UO |
| (O,I,2,0) or (O,I,9,0) | return 2 | LO,LI,R,C,UI,T(2),UO |
| (O,I,0,1) or (O,I,0,2) | return 101 | LO,LI,R,mark20,C,UI,T(0),UO |
| (O,I,1,1) | return 200 | LO,LI,R,mark20,C,UI,T(1),UO |
| (O,I,2,2) | return 97 | LO,LI,R,mark20,C,UI,T(2),UO |
| (O,I,0,3) | throw original ISE("row") | LO,LI,R,C,UI,UO |
| (O,I,0,4) | throw original Error | LO,LI,R,C,UI,UO |
| (O,I,0,5) | throw normal-cleanup IAE | LO,LI,R,C,UI,UO |
| (O,I,0,6) | throw IAE replacing Error | LO,LI,R,C,UI,UO; no suppression |
| (O,I,0,7) | throw ISE("tail") | LO,LI,R,C,UI,T(0),UO; no second UI/cleanup |
| (O,null,0,0) | throw NPE at inner acquisition | LO,failed LI,UO; no helper/UI |
| (null,I,0,0) | throw NPE at outer acquisition | failed LO; no helper/inner acquisition/release |
| (O,O,1,0) | return 10 | same ordered effects, two reentrant acquisitions/releases |

Natural scopes may have no shared follow; every released path must still enter
its own original tail, or propagate through its original handler. The inner
closing brace must not add release. Unknown capture/entry proof and executable
or unavailable path-specific post-release copies still prevent dependent natural
composition; explicit original code remains available.

The additional transfer subject keeps the original parameter live across a
selected result Phi: flag0 selects value; other flags select value+1. It marks 5
while held, releases, then adds the original value. Thus (O,0,7) returns 14 and
(O,1,7) returns 15, with LO,mark5,UO; (null,0,7) fails before mark/release. Int
addition wraps. When P5 delivers an executable selected-edge copy after release,
that exact phase must remain and natural composition must decline; no copy may
be hoisted merely to retain a synchronized wrapper. If the prepared input does
not establish such an obligation, it is effect evidence only, not a passing
post-release-copy guard. These added expectations precede transfer inspection.

The split expectations precede its first baseline/candidate inspection. No
executed behavioral result is claimed.

## Preparation

```sh
mkdir -p .cache/fixtures/monitor-split-tails
java -cp '.cache/evolution-demo/smali-tools/*' org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/monitor-split-tails/classes.dex \
  quality/fixtures/sync/monitor-split-tails
cp .cache/fixtures/monitor-split-tails/classes.dex \
  quality/fixtures/sync/monitor-split-tails/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/sync/monitor-split-tails/classes.dex \
  --exec '{"cmd":"p6-all","details":true}' \
  --exec '{"cmd":"p9-all","text_only":true}'
```

Prepared with Smali 2.5.2, API 24 and OpenJDK 21.0.12.1. Baseline is the recorded
working tree based on `ebbc003` after selected monitor continuation recovery and
before split-outlet source layout. Both comparisons consume the same prepared DEX.
The candidate retains both natural captures and three separate unlocked tails.
The added transfer input establishes executable P5 outlet copies and still retains
PostReleaseCopy separation with explicit monitors. See review.md for the direct
reading judgment and unrun behavioral execution.
