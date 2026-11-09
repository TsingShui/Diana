# Implicit monitor cleanup and catch binding

## Independent discovery expectations

Independently authored Java explanation and Smali input, with original owner
`Lfixture/MonitorSurface;`. Subjects: counts()I and caught(I)I. No third-party
implementation or external declarations are consumed by Diana.

counts retains the reported pattern: enter this monitor; two field reads and
size calls protected by a catch-all; compute the pure sum while held; normal
release and value return; private catch capture, matching release and exact
rethrow. The protected range excludes the sum and both monitor exits. A natural
surface must preserve the field/call order and the sum before releasing the lock,
without executing a second release or showing the compiler cleanup as user code.

Model: the receiver is non-null and initially unlocked by this invocation. Its
fields refer to queues A and S; A.size records A and returns2, S.size records S
and returns3. Either field may be null, so the following original virtual call
throws NPE without executing a modeled size body. A or S may throw a distinct E
instead. No queue read is assumed pure, and the second field is read only after
the first size completes. Monitor acquisition/release follow the existing P2/P6
current-thread ownership model; no asynchronous exception is assumed.

For caught, read(mode) records R(mode), returns mode+7 for mode0 or throws the
same supplied E for mode1. note(E) records N(E), then returns or throws a distinct
selected note error. The capture is used only in the selected catch body. The
target surface is catch(all error), preserving the untyped DEX clause and exact
captured value while moving only its binding into the header.

| Scenario | Expected outcome | Required effects |
|---|---|---|
| counts normal | return5 | acquire this, A, S, release this, return5 |
| A fails | exact E | acquire this, A, release this, throw E; S skipped |
| S fails | exact E | acquire this, A, S, release this, throw E |
| async field null | NPE | acquire this, release this, throw NPE; neither size body runs |
| sync field null | NPE | acquire this, A, release this, throw NPE; S body skipped |
| caught(0) | return7 | R(0); no note |
| caught(1) | return-1 | R(1), N(E), return-1 |
| caught note fails | exact note error replaces E | R(1), N(E), throw note error |

No suppression, retry, copy or extra cleanup occurs in the monitor handler. All
failure paths release exactly the one acquired monitor. Return evaluation and
unlocked work remain at their original locations; no implicit dispatcher is
introduced.

## Initial reproduction status

Expectations precede assembly and candidate inspection. Baseline is main
73fc879. Prepare once and compare the same DEX via ordinary Engine dbg_shell.
At expectation authorship, preparation, inspection, repair and semantic review
had not run. No full-method VM execution is claimed.

## Preparation and current evidence

Prepared on 2026-11-09 with locally available Smali 2.5.2, API24 and Temurin
OpenJDK 21.0.12.1+1. Java is an explanation and Smali the actual assembly input.
The published classes.dex is the same input for baseline73fc879 and the repair.

```sh
mkdir -p .cache/fixtures/implicit-monitor-cleanup
java -cp '.cache/evolution-demo/smali-tools/*' org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/implicit-monitor-cleanup/classes.dex quality/fixtures/sync/implicit-monitor-cleanup
cp .cache/fixtures/implicit-monitor-cleanup/classes.dex quality/fixtures/sync/implicit-monitor-cleanup/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/sync/implicit-monitor-cleanup/classes.dex \
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
