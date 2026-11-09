# Fixture: typed-catch-shared-cleanup

## Current verdict — whole-operation follow-up

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
