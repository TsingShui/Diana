# Fixture: complex-exceptions

## Natural catch display

Only `parseOrThrow` changes. Its natural `NumberFormatException` catch already
prints both routes, so the separate chain line, inline markers and `H3` label are
omitted. `trim`, `parseInt`, catch-body list/log mutation, constructed exception
and throw order are text-identical. P2–P6 source products are exact. Text/JSON publication changes together; the original sites, chains,
completions, snapshots and receives remain in the owned product. This is a
renewed LLM reading of the exact before/after text plus product equality, not
executed behavior.

## Prior verdict — scoped catch capture

Compared the same prepared input on `c95c6dd` and this repair. Reviewed header
bindings: `NumberFormatException v9`, `NullPointerException v8`, `RuntimeException v7`, `all v9`, `NumberFormatException v4`, `all v6`, `all v7`. The original captured object is bound once at exceptional
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

## Prior verdict — terminal guard follow-up

Read resource closed checks and mixed parse/Number conversion. Throw paths
remain before field reads; normal cast/intValue remains inside the same typed
try with original site chains and check order. Resource cleanup code and routes
are unchanged.

Compared the same prepared DEX on 29f1b82 and this commit. P7 requires a complete
linear arm ending in a printed return/throw; opaque scopes are not exit evidence.
The [fresh guard challenge](../../branches/terminal-guard-arms/review.md) covers
actual other-edge Phi copies, parent scope, recovering catches and cleanup.
Original P2 through complete P6 products agree; this is bounded LLM path reading,
not behavioral execution. Historical execution claims below were not rerun.


## Prior verdict — while/do-while architecture correction

Read every changed loop's entry test, ordinary body and original backedge. The
same body work precedes the now-visible update; an exiting test or abrupt effect
skips exactly the original remaining work. Initial values, loop nesting,
call/access/argument order and selected results remain. Some body-only
temporaries can now be declared at their existing assignments because no header
reads them. This changes no original value or effect.

Compared the same prepared DEX on 8ac6d95 and this commit. Dedicated for recovery
contradicted the approved architecture and is removed, not retained behind a
switch. Updates stay in ordinary body positions with original targets/copies;
explicit gotos can be necessary instead of update-skipping while continues.
The [fresh challenge](../../loops/while-tail-continuations/review.md) covers
three-level private continuations, real shared entrances, cleanup and do-while.
This is bounded LLM source/path reading, not behavioral execution. Historical
execution claims below were not rerun and do not certify this output.


## Prior verdict — whole-operation follow-up

Read changed resource/protected arithmetic placement against the Java
explanation and existing DEX facts. Nonthrowing local sums can sit outside a
smaller try without changing their execution relative to close.
Acquisition/read/close order, nested catch priority, selected return values and
cleanup failure replacement remain. No new external resource declaration or
helper contract is required.

The [pure-tail challenge](../../exceptions/pure-tail-finally/review.md) and
[carrier challenge](../../atoms/refined-carriers/review.md) record the owning
producer/consumer repairs and blocked drafts. This is bounded current source
reading; historical execution claims below were not rerun and do not certify
this output. No behavioral executor, device run or Java recompilation ran.


Nested finally, caught rethrow with a cause, resources inside a loop and
multi-catch. `ComplexExceptions.java` is the authored reference; the committed
DEX was prepared with `javac --release 8 -g` and `d8 --min-api 24` through
`quality/build.sh`.

- Provenance: authored for this repository; no third-party code or data.
- Regenerate from the sources; never edit the DEX by hand.

## Prior verdict — round 3

The resource method and its helper were compared directly with the current
ordinary Engine output at `650dade`:

```sh
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/composite/complex-exceptions/classes.dex \
  --exec '{"cmd":"p9-all","text_only":true}'
```

### Semantics

These are manual source/output reading judgments, not automated execution results.
R3 focuses on `Lfixture/ComplexExceptions;->resources(I)I`; the other entries
retain their previous manual judgments.

| Method | Semantics |
|---|---|
| `parseOrThrow` | Previously judged equivalent: parsing, the bad-input log and the rethrow cause agree. Construction remains explicitly split. |
| `nestedFinally` | Previously judged equivalent: inner and outer cleanup execute on their applicable normal/exceptional paths; the arithmetic catch returns -1. |
| `resources` | Equivalent by direct manual path comparison: acquisition, read/update, close, increment and exceptional cleanup agree. |
| `multiAndInstance` | Previously judged equivalent: the Number branch and the -2/-3 catch results agree. Explicit cast/owner notation remains. |

### Resource path comparison

| Path | Reference and recovered behavior |
|---|---|
| `count <= 0` | Return 0 without constructing or closing a resource. |
| Successful iteration | Construct `Res(index)`, call `read()`, update total, close that resource, then increment index. |
| Normal result | Reads contribute `2 * index` with the same integer arithmetic. `resources(3)` yields 6. |
| Construction fails | Propagate without closing that unacquired resource or starting another iteration. Earlier iterations have already closed their resources. |
| Read fails with E | Capture E, attempt close, then rethrow E; no total update, increment or later iteration occurs. |
| Read fails with E and close fails with C | Call `E.addSuppressed(C)` and rethrow E, assuming distinct identities and a normally returning `addSuppressed`. |
| Normal close fails | Propagate the close failure; do not enter the read-failure catch, retry close or increment index. |

The recovered while loop has the same entry test and update position as the
source for loop. Normal close is outside the read-protection try; exceptional
close and suppression remain inside its catch. The helper still tests `closed`
and returns `id * 2`; close still sets the same field.

The fixture's concrete close only writes its flag, and a newly constructed
resource is read before close. The throwing cases above describe a manual
comparison of exception routing, not injected faults exercised by this input.
Automated behavioral execution has not been performed.

### Readability

- Resource handlers are now inside the loop body, including the nested
  close/suppression catch. This method needs no detached handler or explicit
  exception chain.
- `nestedFinally` has a natural inner try, but the outer mixed catch/finally
  protocol still requires following explicit exception chains.
- Split construction, `via` disclosure, explicit runtime casts and field-load
  temporaries remain elsewhere in the class. They are not evidence of an
  execution mismatch by themselves.

### Ranked fixes

Make the remaining outer finally protocol easier to follow. Improve expression
and construction spelling only when original evaluation and check order remain
visible.

## Rounds

- R3: Direct manual reading finds resource behavior consistent with the source;
  automated behavioral execution remains separate and unrun.
- R2: Resource cleanup moved into nested try/catch inside the loop; the manual
  semantic judgment was left pending.
- R1, 2026-11-08: Detached resource handlers obstructed
  reading; this was not an independently established behavioral mismatch.
