# Fixture: typed-catch-shared-cleanup

## Natural catch display

The outer union/RuntimeException/finally shape prints without chain text or labels,
including the three constructed throwables and `parse`. The catch body's own
`open`/`read`/`close` inner scope, its result commits and the `-2` recovery stay
text-identical, so the shared-cleanup and rejection guards are still stated. Text and JSON publish together; owned sites, chains, completions, snapshots
and receives remain. Renewed LLM reading and product equality, not executed
behavior.

## Current verdict — scoped catch capture

Compared the same prepared input on `c95c6dd` and this repair. Reviewed header
bindings: `NumberFormatException | NullPointerException v11`, `all v16`, `RuntimeException v17`. The original captured object is bound once at exceptional
entry, before the unchanged receiving/body actions. Its exact-state uses remain
inside that handler and have no capture-phase, Phi or copy dependency requiring
an outer assignment. Normal execution does not bind or run the handler.

After accounting for that binding and its removed private declaration/assignment,
the complete remaining output is exact. Original P1/P3/P4/P5 products agree.
Thus the earlier independently reasoned normal, caught, cleanup, rethrow and
secondary-failure paths below are reused: their calls, arguments, values,
priority, transfers and effect order did not change. The renewed LLM judgment
finds no contradictory capture lifetime or exception identity. Structural
comparison supports this reading rather than supplying behavioral proof.

The [fresh monitor/binding challenge](../../sync/monitor-surface-boundaries/review.md)
covers capture identity, receiving/value flow, nested failures and escaping
storage. Current fixture/model regressions are separate executed evidence;
this record does not claim VM/device execution for this input.

## Prior verdict — constructor argument and throw recovery

Compared the same prepared DEX on `e95b6a4` and the containing repair. Read the
original Java and prepared DEX behavior and complete before/after output for classify(String,I)I.
Only construction spelling changes; other member output is exact.

### Semantics

Modes1/2/3 now print natural NFE(mode), NPE(mode) and ISE(mode) throws while retaining the same ordered c1 routes. The adjacent NFE/NPE handler still opens resource1, reads/adjusts its result and closes it; RuntimeException still selects -2. Normal parse and all outcomes retain the outer close of resource0. Constructor/string faults use original type dispatch and are not fabricated as the intended exception. Shared handler priority and inner/outer cleanup effects remain exact.

Original P1/P3/P4 publications agree. P5 retains allocation, arguments and normal
publication with the same ordered protection; P7 composes an adjacent throw only
with equal handler chain/spelling and no snapshots. Original sites and effect
order stay delivered, including explicit chains when lexical catch delivery is
unavailable. The [fresh argument/effect challenge](../../atoms/constructor-argument-effects/review.md)
covers trees, fault prefixes and order/sharing/handler/save refusals.

LLM path review found no contradiction under the fixture's recorded helper and
external-constructor assumptions. Unchanged paths reuse the earlier independent
review below. This is renewed semantic reading plus current product regressions,
not a full-method VM/device or Java execution result. Historical execution claims
below are not rerun certifications.

## Prior verdict — terminal guard follow-up

Read mode-selected constructions inside the original inner try. NFE/NPE/ISE
still occur before parse and keep their site chains/priority. Only normal mode
reaches parse/assignment; shared recovery, opens/closes, captured return and
outer recovery remain unchanged.

Compared the same prepared DEX on 29f1b82 and this commit. P7 requires a complete
linear arm ending in a printed return/throw; opaque scopes are not exit evidence.
The [fresh guard challenge](../../branches/terminal-guard-arms/review.md) covers
actual other-edge Phi copies, parent scope, recovering catches and cleanup.
Original P2 through complete P6 products agree; this is bounded LLM path reading,
not behavioral execution. Historical execution claims below were not rerun.


## Prior verdict — whole-operation follow-up

Read typed read/recovery, nested resource and outer recovery paths. NFE recovery
retains its original open/read/close and +100 result before cleanup; close
failure or rethrow reaches the original outer handler where applicable. Normal
and captured return values are selected before close. Pure arithmetic placement
changes no throwing site, handler priority or cleanup order.

The [pure-tail challenge](../../exceptions/pure-tail-finally/review.md) and
[carrier challenge](../../atoms/refined-carriers/review.md) record the owning
producer/consumer repairs and blocked drafts. This is bounded current source
reading; historical execution claims below were not rerun and do not certify
this output. No behavioral executor, device run or Java recompilation ran.


## Prior verdict — round 4

The candidate's enclosing cleanup now prints as finally. The original normal
result copies still execute before close(50); two executable incoming-copy spans
remain before the finally header. NFE/NPE detail handling, the separate broad
RuntimeException handler, nested close(51), and result values remain unchanged.
The same read/close fault paths reviewed below still close 51 before 50 and
replace, rather than suppress, pending failures. Main acquisition remains outside
protection, and no failed cleanup is retried.

The owning-stage repair uses the selected catch-all suffix, not the full source
chain. It compares existing value-preserving numeric views without discarding
their original declaration origins, and does not treat body outlet copies as
execution inside cleanup. `../branching-finally-suffix/` supplies the new loop,
branching cleanup, unequal-argument and outside-rethrow challenges. Its safety
guard exposed a pre-existing implicit-rethrow route loss, now prevented by
retaining the explicit original dispatch when enclosing delivery is not proved.

This is an LLM path comparison plus structural/product evidence, not executed
behavioral acceptance. The detail cleanup remains explicit because guarded
propagation and its surrounding ownership are not part of this finally alternative.

## Prior verdict — round 3

Direct LLM comparison of the Java explanation, Smali and ordinary Engine output
on the candidate based on `ebbc003`. The native helper models and independent
scenarios are in README; their bodies are not present in the DEX. This is semantic
reading, not behavioral execution.

```sh
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/typed-catch-shared-cleanup/classes.dex \
  --exec '{"cmd":"p6-all","details":true}' \
  --exec '{"cmd":"p9-all","text_only":true}'
```

### Semantics

| Original method | Judgment |
|---|---|
| `Lfixture/TypedCatchSharedCleanup;->classify(Ljava/lang/String;I)I` | Equivalent on reviewed paths: original catch priority, detail acquisition/cleanup, shared outer cleanup, results and exception replacement agree. |

Normal parse stores its result before close(50), then returns 7 in the default
model. NFE/NPE select one shared handler: open(1) returns 51, read(51) returns 11,
the result becomes 111, close(51) runs, then close(50). The broader RuntimeException
clause remains second and selects -2. Error bypasses both typed catches, closes
50 and rethrows the original object. A first acquisition failure occurs before
all protection and performs no close.

Detail acquisition failure enters only the outer cleanup. Detail read failure
enters the nested catch-all close and rethrow. A failing close(51) replaces that
read failure rather than suppressing it; the outer close(50) still runs. A failing
outer close replaces the pending result or exception, with no retry. The shared
normal close and the outer handler's close are outside the protection they
implement. The output preserves these distinctions and each captured identity.

### Recovery mechanism

The baseline rejects repeated handler targets and prints the typed catches and
outer cleanup as detached labels. The candidate publishes contiguous original
clause groups, retaining separate types/priority/edges with one body and capture.
It also recognizes the observed outer suffix as its own protection layer and
includes nonthrowing control prefixes without absorbing unguarded calls.

The result has nested natural try regions: one NFE/NPE compound catch, then
RuntimeException, inside the enclosing catch-all cleanup. The detail read retains
its own catch-all. The full original chain remains inspectable and some markers
remain visible for multi-route sites. Catch-all is not relabeled finally without
an established cleanup correspondence.

### Complexity challenge and regression

`../typed-catch-loop-exits/` was authored after the initial repair. It adds a
loop, normal continue/break/return decisions, concrete throwing helpers and
cleanup replacement. Its candidate retains the compound catch and enclosing
cleanup; nonadjacent aliases stay explicit. A loop exposed another same-cause
boundary: later outer clauses must remain excluded from private catch membership
even when a backedge makes them reachable again.

Three checks in `crates/diana-decomp/tests/catch_groups.rs` cover selected ownership,
original clause coverage, priority guards, capture/call anchors and the challenge.
The existing 19 structural checks were also run successfully during this repair.
They do not independently execute the fixture scenarios.

Before/after inspection of 76 fixtures, 126 classes and 587 methods preserved P2,
P3/P4 and original storage/control/exception/lock facts, with no stopped member,
render failure or render issue. Six fixture outputs changed; changed paths were
read against their source. Unchanged input/output reviews are reused, not rerun
behavioral observations. Constructor/VM failures outside the README model and
concurrent helper effects remain outside the reviewed scenarios.

## Ranked fixes

The challenge exposed private handler rejoins defeating normal loop dominance.
The following loop iteration now establishes their closed real entry without
changing normal dominance. `../nested-catch-rejoins/` records the required nested
monitor challenge and genuine foreign-arrival guard. Catch ownership and all
original transfers remain intact. Cleanup is still explicit catch-all where no
published finally correspondence has been established.

## Rounds

- R3: Shared ordered clauses, nonthrowing prefixes and inherited cleanup scopes
  recover natural catches; complexity challenge confirms the same guarantee.
- R2: Parent semantic reread corrected acquisition-ID versus returned-handle
  notation in the expectations and identified the repeated-target producer barrier.
- R1: Initial manual reading found agreement under the helper model, with explicit
  chains and detached handlers as the main readability obstruction.
