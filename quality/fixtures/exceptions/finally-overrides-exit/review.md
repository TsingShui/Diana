# Fixture: finally-overrides-exit

## Current verdict — abrupt finally recovery

Read original Java/Smali and both complete `replaceExit(I)I` texts on baseline
`b214ec6` and this repair. Duplicated close/override paths now become one finally;
body construction/read remain in their original order.

### Semantics

| Path | Source and recovered behavior |
|---|---|
| Acquisition | `open(0)` remains before try. An open failure never reaches close. With the native-helper model, handle7 is passed to read and close. |
| Ordinary body | Bit1 clear reaches read once; ordinary cleanup closes once. With bits2/4 clear, return5 remains after cleanup. |
| Body error | Bit1 constructs and throws ISE(`body`) before read. A read failure is reachable with bit1 clear. Both errors enter cleanup with the original acquired handle. |
| Cleanup override | Close precedes both tests. Bit2 returns -7, including mode6; bit4 is reached only with bit2 clear and constructs/throws IAE(`finally`). Either replaces the pending return or exact captured error. |
| Cleanup completion/error | With no override, a captured body/read error is implicitly rethrown with its identity. Close failure propagates outside its original protected range, skips both overrides and replaces the pending exit without suppression or retry. |

P6 pairs explicit cleanup terminals and the selected P5 natural construction;
P7 retains canonical exits and zero-width duplicate/capture/rethrow origins.
Original body construction remains explicit because its own P5 group is unchanged.
All original P1/P3/P4/P5 publications agree. The new
[complexity challenge](../abrupt-finally-completion/review.md) covers saved body
state, throwing close, void/all-abrupt cleanup, multiple rethrows and refusal
boundaries. Review found no contradiction in the independently recorded scenarios.
Native helpers use the README model; asynchronous exceptions and allocation or
external constructor failure remain outside scenario execution. This is LLM
semantic review plus product/origin tests, not a VM behavioral result.

## Prior verdict — terminal guard follow-up

Read complete replacement paths. Body bit1 throws before read. Each cleanup
closes first, bit2 returns-7, otherwise bit4 throws the cleanup IAE, otherwise
returns5 or rethrows the original capture. Failures are neither retried nor
suppressed; no guard moves across the enclosing try/catch.

Compared the same prepared DEX on 29f1b82 and this commit. P7 requires a complete
linear arm ending in a printed return/throw; opaque scopes are not exit evidence.
The [fresh guard challenge](../../branches/terminal-guard-arms/review.md) covers
actual other-edge Phi copies, parent scope, recovering catches and cleanup.
Original P2 through complete P6 products agree; this is bounded LLM path reading,
not behavioral execution. Historical execution claims below were not rerun.


## Prior verdict — whole-operation follow-up

Reread Java, Smali and complete final replaceExit. Body construction/read and
original catch-all remain. On normal and caught paths close executes first; bit2
then returns -7, otherwise bit4 throws the finally IAE, otherwise return5 or
original rethrow completes. Close failure replaces the pending exit and is not
retried. Constructor preparations remain original explicit operations when no
complete natural group is established.

The [pure-tail challenge](../../exceptions/pure-tail-finally/review.md) and
[carrier challenge](../../atoms/refined-carriers/review.md) record the owning
producer/consumer repairs and blocked drafts. This is bounded current source
reading; historical execution claims below were not rerun and do not certify
this output. No behavioral executor, device run or Java recompilation ran.


An explicit finally whose close, return and throw replace a pending body exit.
`FinallyOverridesExit.java` explains the mode-bit behavior;
`FinallyOverridesExit.smali` is the assembly input. The prepared `classes.dex`
was assembled from the Smali and copied beside it.

- Provenance: authored for this repository; no third-party code or data.
- Original owner: `Lfixture/FinallyOverridesExit;`.
- Subject: `Lfixture/FinallyOverridesExit;->replaceExit(I)I`.
- R1 review base: repository HEAD `6d663d9`; engine sources unchanged from
  `12de370`.
- Expected behavior: the scenario table in `README.md`.

## Prior verdict — round 2

Manually reread the Java explanation, Smali and fresh P6/P9 delivery at
`09e322b`. The protected body routes to H15; its close, bit-2 return and bit-4
throw preserve replacement order. A failing close precedes both override checks.
A finally return overrides the pending primary; a finally throw replaces it
without suppression. No semantic mismatch was found on the reachable paths.

The README had an unreachable fault scenario: mode 3 throws the body ISE before
the read. The read-failure/finally-return scenario now uses mode 2, derived from
the source bit tests, while the separate mode-3 body-throw scenario remains.
A failing close also prevents construction of the finally IAE; it does not
discard an already-created IAE.

The current text is unchanged from the inspected `6d663d9` engine. P6 delivers
zero natural try items, so the finally still appears as normal code plus H15.
This is a readability obstruction. Fresh delivery has no stopped member or
rendering issue. Behavioral execution remains unrun.

## Prior review — round 1

### Semantics

This is a manual source/output reading judgment, not an automated execution result.

| Method | Semantics |
| --- | --- |
| `replaceExit` | ~ Equivalent behavior with a different shape: the finally copies run on the right paths in the right order, but no `try` or `finally` text is recovered. |

### Path comparison

The output has two finally copies: one after the normal branch and one in `H15`.
The normal branch selects the body result or the body throw; `H15` closes and then
applies the same return/throw overrides to the pending exit.

| Path | Reference and recovered behavior |
|---|---|
| `mode & 1 == 0` and no override | O(0), R(7), C(7), return 5. The output prints the read, close and `return 5` in that order. |
| Body throw | The constructed `IllegalStateException("body")` is thrown; `exception_chain c1` routes it to `H15`; `H15` captures it, calls C(7), then applies the finally. |
| `mode & 2 != 0` | The first finally check returns -7 before the bit-4 throw. The output returns -7 on both the normal branch and `H15`. |
| `mode & 4 != 0` | The finally constructs `IllegalArgumentException("finally")` after the close and throws it. |
| Body throw plus bit 2 | `H15` returns -7 and discards the pending `ISE`. |
| Body throw plus bit 4 | `H15` throws the new `IAE`; the pending `ISE` is discarded without a suppression call. |
| Helper close failure | C(7) is outside the protected range in both finally copies. It therefore replaces the pending return or exception instead of being suppressed. `H15`'s close failure also leaves `H15` without a retry. |
| Helper read failure | The read is protected by `c1`; `H15` closes and applies the finally overrides, including returning -7 or throwing `IAE` in place of the read failure. |

An `addSuppressed` failure is not applicable: this fixture has no suppression
call. The constructed message strings and throwable types are preserved by the
split `new-instance`, `const-string`, constructor-call and throw sequence.

### Concrete mismatches

None found in the reviewed paths. The absence of a `try`/`finally` spelling is a
shape difference, not an execution mismatch.

### Reading obstruction

The method body reads as an `if`/`else` followed by an `H15:` block. The body
throw is inside the `else`; `H15` is reached only through `exception_chain c1`,
and every protected operation carries `[exn c1]`. A reader must understand that
`H15` is the duplicated finally, that its close failure is intentionally outside
its own protected range, and that its bit-2/bit-4 branches override the captured
exception. The handler block is not printed inside a `catch` or `finally`
container. This is a concrete readability obstacle, not by itself a semantic
error.

### Uncertain and unreviewed paths

- The native helper calls are modeled by the scenario table; there is no helper
  body in the DEX that can be read independently.
- Constructor failure is outside the scenarios and is not modeled.
- No automated behavioral execution or Rust fixture test was run.

### Commands

```sh
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/finally-overrides-exit/classes.dex \
  --exec '{"cmd":"p9-all","text_only":true}'
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/finally-overrides-exit/classes.dex \
  --exec '{"cmd":"p6-all","details":true}'
```

## Rounds

- R1: Prepared `classes.dex` from the authored Smali; manual path comparison finds
  the finally replacement semantics preserved. The missing `try`/`finally` shape
  and the detached `H15` handler are recorded as a reading obstruction.
