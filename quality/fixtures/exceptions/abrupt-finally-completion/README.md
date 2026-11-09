# Abrupt cleanup, captured returns and explicit guards

## Independent expectations

Authored after the first P6 abrupt-finally repair and before inspecting this
candidate input. Discovery: `FinallyOverridesExit.replaceExit(I)I` in
`exceptions/finally-overrides-exit`. The new challenge keeps paired normal and
catch-all cleanup paths, adding body-return preparation before a first arithmetic
cleanup event, changed cleanup parameters, always-abrupt and void exits,
constructed exceptions and unequal/foreign-entry refusal guards.

All sources are independently authored for this repository. Java explains the
behavior; Smali is the assembly input. Original owner `Lfixture/CleanupExits;`.
Subject prototypes are `capture(III)I`, `always(II)I`, `voidAlways(I)V`,
`bareVoid(I)V`, `parameterThrow(Ljava/lang/Throwable;I)V`, `differentReturn(I)I`,
`differentThrow(I)V` and `foreign(II)I`. Helpers are `mark(I)V`, `read(II)I`
and `close(II)V`, with concrete bodies beside this file.
`twoRethrows(II)I` additionally has two original capture-rethrow terminals and
one shared normal return continuation.

Each scenario starts with trace zero. mark appends an event as wrapping int
`trace * 31 + event`. read(stage,fault) marks 10+stage, then throws a fresh
ISE("read") if the stage bit is set, otherwise returns stage+7. close marks
30+stage, then throws a fresh IAE("close") if the corresponding bit shifted by
four is set, otherwise completes. These subjects use stage 0: fault 1 selects
read failure, fault 16 selects close failure, and fault 17 selects both.
The read/close throwables, constructed override IAE and supplied parameter
throwable are distinct. Constructors and string resolution succeed in the
recorded scenarios; concurrent mutation and asynchronous exceptions are excluded.

| Subject and inputs | Expected exit | Ordered marks |
|---|---|---|
| `capture(0,1,0)` | return 8, not the recomputed 108 | 10,30,101 |
| `capture(2,1,0)` or `capture(6,1,0)` | return -7; return precedes override throw | 10,30,101 |
| `capture(4,1,0)` | throw override IAE | 10,30,101 |
| `capture(0,1,1)` | throw original read ISE | 10,30,101 |
| `capture(2,1,1)` | return -7, discarding read ISE | 10,30,101 |
| `capture(4,1,1)` | throw override IAE, discarding read ISE | 10,30,101 |
| `capture(2,1,16)` or `capture(2,1,17)` | throw close IAE; mark101 and later return/throw are skipped | 10,30 |
| `always(0,0)` or `always(0,1)` | return 8 | 10,30 |
| `always(4,0)` or `always(4,1)` | throw override IAE | 10,30 |
| `always(0,16)` or `always(4,17)` | throw close IAE | 10,30 |
| `twoRethrows(1,0)` | return8 through close0 | 10,30 |
| `twoRethrows(0,0)` | return8 through close1 | 10,31 |
| `twoRethrows(1,1)` | throw original read ISE after close0 | 10,30 |
| `twoRethrows(0,1)` | throw original read ISE after close1 | 10,31 |
| `twoRethrows(1,17)` | throw close0 IAE, replacing read ISE | 10,30 |
| `twoRethrows(0,33)` | throw close1 IAE, replacing read ISE | 10,31 |
| `voidAlways(0)` or `voidAlways(1)` | return void, discarding a pending read failure | 10,30 |
| `voidAlways(16)` or `voidAlways(17)` | throw close IAE | 10,30 |
| `bareVoid(0)` or `bareVoid(1)` | return void, with no close | 10 |
| `parameterThrow(T,0)` or `parameterThrow(T,1)` | throw the exact supplied T | 10 |
| `parameterThrow(null,0)` or `parameterThrow(null,1)` | original throw-null NPE | 10 |
| `differentReturn(0)` | return -7 | 10,30 |
| `differentReturn(1)` | return -8 | 10,30 |
| `differentThrow(0)` | throw IAE("normal") | 10,30 |
| `differentThrow(1)` | throw IAE("handler") | 10,30 |
| `foreign(8,0)` | return8 without entering the body | 30 |
| `foreign(0,1)` | return8 through the catch | 10,30 |
| `foreign(8,16)` | throw close IAE without read | 30 |

No suppression or retry occurs. All cleanup failures replace a pending body exit.
The normal and handler return/message differences in the refusal cases are
intentional and are not one finally. The foreign entry shares the actual normal
cleanup from outside the protected interface, so it remains explicit.
Capture's pure body add and cleanup add have different original reads: instruction
kind alone cannot establish the first cleanup event. Its saved result stays
before cleanup, even when the base changes and the return prints after finally.

## Preparation and inspection

Prepared on 2026-11-09 with the locally available Smali 2.5.2 assembler,
`--api 24`, and Temurin OpenJDK 21.0.12.1+1. Reassembly from the committed Smali
was byte-identical to the published DEX. No assembler is needed for inspection.
DEX SHA-256: `414aa886b0d677f2b592aa0b3119ea2ca7f61366482c4a2721b7a0290bea3ae6`.

```sh
mkdir -p .cache/fixtures/abrupt-finally-completion
java -cp '.cache/evolution-demo/smali-tools/*' org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/abrupt-finally-completion/classes.dex \
  quality/fixtures/exceptions/abrupt-finally-completion
cp .cache/fixtures/abrupt-finally-completion/classes.dex \
  quality/fixtures/exceptions/abrupt-finally-completion/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/abrupt-finally-completion/classes.dex \
  --exec '{"cmd":"p5-all","details":true}' \
  --exec '{"cmd":"p9-all","text_only":true}'
cargo nextest run --locked -p diana-decomp --test abrupt_finally
```

## Review and regression record

The same prepared input was inspected on baseline `b214ec6` and the containing
repair. Six subject methods now select finally; the three intentional unequal
or foreign-entry guards remain explicit. `review.md` compares every subject and
helper path against the authored expectations.

The six grouped tests in `crates/diana-decomp/tests/abrupt_finally.rs` passed.
They check saved body-return state, actual cleanup exits, zero-width duplicate
and pending-rethrow anchors, complete coverage and refusal boundaries through
the ordinary Engine. Full dev nextest ran 121 tests: 121 passed, none skipped,
on Rust 1.98.0 with nextest 0.9.149. Workspace Clippy, formatting and the dev
example build passed; the final integration checks are recorded below.

The initial comparison includes all 99 prior Quality DEX inputs and this new
fixture: 100 inputs, 156 rendered classes and 764 retained methods on both
revisions (703 expressed, 61 declaration-only). No class, member-retention or
render stop occurred. Output is byte-exact for 95 inputs. The changed set is
`composite/complex-flow`, `exceptions/exception-finally-transfer`,
`exceptions/finally-overrides-exit`, `obfuscated/obf-flow-r8`, and this fixture.
Every changed input retains identical P1/P3/P4/P5 publications and has a renewed
fixture-local LLM review. Prior reviews are reused for unchanged input and output.
These counts describe coverage, not quality ratings or semantic proof.

No complete-method behavioral executor, device run or Java recompilation was
implemented or run for this repair. Existing implemented regression tests run
separately; the written review is not executed behavioral PASS. Release regression
was not run, following the operator's dev-only verification instruction.

## Integrated dev verification

Rebased the repair onto main `5811655` without conflicts, preserving the concurrent
Kotlin and GUI changes. On that combined source, all 126 nextest tests passed
with none skipped. `cargo fmt --all -- --check`, workspace Clippy with all targets
and features, and `cargo build --workspace --bins --examples --locked` passed.
These builds use dev, following the operator's instruction.

Compared all 100 Quality DEX inputs present in that main revision plus this new
fixture on `5811655` and the containing repair. Both revisions render 161 classes
and retain 778 methods (715 expressed, 63 declaration-only), with no class,
member-retention or rendering stop. The 96 unchanged inputs have byte-exact P9
text; the five changed inputs are the same set listed above. Their current
P1/P3/P4/P5 publications remain identical between revisions, and their candidate
P9 text is exact to the already reviewed candidate output, so those path reviews
are reused for the combined source. No expectations or regression cases changed.

Dev CLI `get-method` additionally confirmed matching text/JSON result text and
one finally for the original discovery, `capture(III)I` and `bareVoid(I)V`.
The desensitization gate passed. No release or full-method VM behavioral result
is implied by these checks.
