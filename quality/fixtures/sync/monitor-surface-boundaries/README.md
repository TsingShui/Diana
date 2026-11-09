# Monitor surface boundaries

## Independent complexity challenge

Authored after the initial repair of sync/implicit-monitor-cleanup and before
candidate inspection. Original owner Lfixture/MonitorBoundaries; with nested
(Object,Object,I)I, innerCatch(Object,I)I, extra(Object,I)I,
otherException(Object,Throwable,I)I, twoReturns(Object,I)I,
wrongLock(Object,Object,I)I and escaped(I)Throwable. Full DEX prototypes are in
Smali; Java describes the normal subjects and deliberate explicit monitor cases.
The explanation's acquire/release notation represents monitor operations only,
not additional assembled/native calls.

The original matched-release/capture/rethrow trigger now has nested acquisitions
(including reentrancy on the same object), an inner typed recovery, an extra
throwing cleanup before release, a replaced exception, distinct normal return
outlets, a different exceptional lock and a capture exported through a Phi.
These cases test actual behavior and support boundaries rather than text similarity.

Models reset per scenario. read(stage,mode) records R(stage), throws a distinct
E(stage) when mode & (1 << stage), otherwise returns stage+2. For innerCatch,
mode1's E(0) is ISE; an independently selected other error is outside that typed
catch. note(error) records N(error), then returns or throws a distinct E(N).
Acquisition of a null reference throws NPE before entering that lock body.
Successful acquisition increments this invocation's owned depth; release of that
same capture decrements it once. A release of a distinct unowned object throws
IME and does not release the held object. No asynchronous exceptions are modeled.

| Scenario | Expected result and ordered lock/call effects |
|---|---|
| nested(O,I,0), distinct non-null locks | return9; acquire O,R0,acquire I,R1,release I,R2,release O,return9 |
| nested(O,O,0) | return9; depths1,2,1,0 in that same order; no repeated or missing release |
| nested(O,I,1) | E0; acquire O,R0,release O,throw E0; I never acquired |
| nested(O,I,2) | E1; acquire O,R0,acquire I,R1,release I,release O,throw E1; R2 skipped |
| nested(O,I,4) | E2; acquire O,R0,acquire I,R1,release I,R2,release O,throw E2 |
| nested(null,I,0) | NPE on initial acquisition; no calls/releases |
| nested(O,null,0) | NPE; acquire O,R0,failed acquire I,release O; no R1/R2 |
| innerCatch(L,0) | return2; acquire L,R0,release L,return2 |
| innerCatch(L,1) | return10; acquire L,R0,typed recovery,release L,return10 |
| innerCatch other read fault | exact error; acquire L,R0,release L,throw error |
| extra(L,1), note succeeds | E0; acquire L,R0,N(E0),release L,throw E0 |
| extra(L,1), note fails | E(N); acquire L,R0,N(E0),throw E(N); L remains held in this deliberately leaking input |
| otherException(L,T,1) | exact T after release; acquire L,R0,release L,throw T |
| otherException(L,null,1) | NPE at original throw after release, not before it |
| twoReturns(L,0) | return2 after its own release |
| twoReturns(L,2) | return102; R0 succeeds, update100 precedes its other release and unlocked return |
| twoReturns(L,1) | E0 after the exceptional release |
| wrongLock(A,B,1), distinct B unowned | IME replaces E0, A remains held; no invented release A |
| wrongLock(A,A,1) | E0 after one release A; missing equality facts must not assume this scenario |
| escaped(0) | null after R0 |
| escaped(1) | exact E0 after R0; handler-to-join Phi/copy must retain the captured object |

The extra and wrong-lock inputs cannot be expressed as a normal Java synchronized
block without changing failure behavior. Their explicit operation notation is
intentional and Smali is authoritative. OtherException preserves release before
its replacement throw; twoReturns preserves two separate original unlocked
returns. Escaped's capture value remains observable outside its catch scope.

## Initial status

Expectations precede DEX preparation, inspection and review. Compare the same
prepared input on baseline73fc879 and the repair through ordinary dbg_shell.
No VM/device or full-method behavioral execution is claimed at authorship.

## Preparation and current evidence

Prepared on 2026-11-09 with locally available Smali 2.5.2, API24 and Temurin
OpenJDK 21.0.12.1+1. Java is an explanation and Smali the actual assembly input.
The published classes.dex is the same input for baseline73fc879 and the repair.

```sh
mkdir -p .cache/fixtures/monitor-surface-boundaries
java -cp '.cache/evolution-demo/smali-tools/*' org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/monitor-surface-boundaries/classes.dex quality/fixtures/sync/monitor-surface-boundaries
cp .cache/fixtures/monitor-surface-boundaries/classes.dex quality/fixtures/sync/monitor-surface-boundaries/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/sync/monitor-surface-boundaries/classes.dex \
  --exec '{"cmd":"p5-all","details":true}' \
  --exec '{"cmd":"p9-all","text_only":true}'
cargo nextest run --locked -p diana-decomp --test sync_surface
```

The original counts/caught and the fresh nested/innerCatch inputs expose the
recorded obstruction on the pre-fix engine. Six grouped ordinary-Engine checks
passed on the candidate. The test-owned final-plan model executed 20 documented
scenarios across counts, caught, nested, innerCatch and escaped, checking results,
ordered field/call/lock effects and exact errors. It supports only the fixture's
control/expression/helper subset; unknown forms fail checking. Extra-work,
changed-throw, split-return and different-lock guards have source/product/origin
review, not executed VM observations. No general JVM/device run is claimed.

Primary fixture-local LLM review is in review.md. The existing capture/header
and monitor-anchor checks were updated to recognize the new surface while
retaining caught-type, effect/control and original-owner constraints. The full
dev nextest set passed 152 tests with none skipped on Rust 1.98.0. Formatting,
workspace Clippy and a dev binary/example build are integration checks. Release
regression is not required by the operator's standing instruction.

The initial same-input comparison includes 108 previous Quality DEX inputs plus
the two new inputs: 110 fixtures, 176 classes and 845 retained methods on each
revision (760 expressed, 85 declaration-only). No class/member/render stop occurs.
74 outputs are byte-exact; 36 changed fixtures receive renewed capture/monitor
reading. P1/P3/P4/P5 products remain exact. Header-only changes reuse earlier
independent path reviews after checking that handler effects/transfers and value
use remain unchanged; monitor changes receive explicit lock-order review. These
counts describe coverage rather than quality ratings or whole-program proof.

## Integrated dev verification

Rebased onto main `c95c6dd`, preserving concurrent terminal-follow conjunction
code and both sets of fixture reviews. On the combined source, 155 nextest tests
passed with zero skips, including the 20 documented fixture model scenarios.
Workspace format, Clippy with all targets/features and the dev binary/example
build passed. Dev CLI get-method confirmed equal text/JSON result text on counts,
caught, nested and innerCatch, with natural synchronization/header capture.

The final same-input comparison includes 109 prior Quality inputs plus these two
fixtures: 111 DEX inputs, 177 rendered classes and 851 retained methods on each
revision (766 expressed, 85 declaration-only), with no class/member/render stop.
75 outputs are byte-exact; the same 36 changed fixtures have renewed reviews.
P1/P3/P4/P5 remain exact. The concurrent condition changes are shared by both
comparison revisions; handler-body comparison reuses the newest prior reviews.
The desensitization gate passed. No release regression or general VM result is
implied by these dev/model observations.
