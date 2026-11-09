# Fixture: obf-exceptions-proguard

## Natural catch display

Only obfuscated `a(String)` changes in the same way as its reviewed source
counterpart: the natural `NumberFormatException` catch prints without a repeated
chain line, marker or `H3`. `trim`, `parseInt`, catch-body mutation and the
constructed exception order are text-identical. P2–P6 source products are exact. Text/JSON publication changes together; the original sites, chains,
completions, snapshots and receives remain in the owned product. This is a
renewed LLM reading of the exact before/after text plus product equality, not
executed behavior.

## Prior verdict — scoped catch capture

Compared the same prepared input on `c95c6dd` and this repair. Reviewed header
bindings: `all v8`, `NumberFormatException v8`, `NullPointerException v7`, `RuntimeException v6`, `NumberFormatException v4`, `all v6`, `all v7`. The original captured object is bound once at exceptional
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

Read closed/read and mixed parse/Number paths with original obfuscated
identities. The early return/throw skips only the original other work;
check-cast/intValue still share the same typed try, original sites and order.
Cleanup schedules remain unchanged.

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

Read the changed protected arithmetic/resource paths against the retained
explanation/mapping. Original obfuscated identities, acquisitions, reads,
reverse cleanup order and pending fault delivery remain. Pure scalar placement
outside a smaller catch does not cross a cleanup effect or fabricate a result
after failure.

The [pure-tail challenge](../../exceptions/pure-tail-finally/review.md) and
[carrier challenge](../../atoms/refined-carriers/review.md) record the owning
producer/consumer repairs and blocked drafts. This is bounded current source
reading; historical execution claims below were not rerun and do not certify
this output. No behavioral executor, device run or Java recompilation ran.


ProGuard-obfuscated nested finally, resource cleanup inside a loop and
multi-catch. The Java reference is authored for this repository; no third-party
program or data is used.

- `rules.pro` selects the local obfuscator used by `quality/build.sh`.
- `mapping.txt` pairs original names and full signatures with the obfuscated
  declarations. Overloaded short names alone do not identify a method.
- Regenerate from the sources; never edit the DEX by hand.

## Prior verdict — round 3

The Java resource method and helper were compared directly with current ordinary
Engine output at `650dade`, using the mapping only for this written review:

```sh
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/obfuscated/obf-exceptions-proguard/classes.dex \
  --exec '{"cmd":"p9-all","text_only":true}'
```

### Semantics

These are manual source/output reading judgments, not automated execution results.
R3 focuses on the mapped resource method; other entries retain their previous
manual judgments.

| Reference method | DEX identity | Semantics |
|---|---|---|
| `parseOrThrow` | `La/a;->a(Ljava/lang/String;)I` | Previously judged equivalent: parsing, logging and the rethrow cause agree. |
| `nestedFinally` | `La/a;->a(I)I` | Previously judged equivalent: inner and outer cleanup retain their normal/exceptional paths. |
| `resources` | `La/a;->b(I)I` | Equivalent by direct manual path comparison: loop order, one close per successful iteration, primary exception and suppression agree. |
| `multiAndInstance` | `La/a;->a(Ljava/lang/Object;)I` | Previously judged equivalent: the Number branch and catch results agree. |

### Resource path comparison

The mapping identifies the resource as `a.b`, `read()` as `a()` and `close()` as
`close()`. The recovered `b(I)I` performs:

1. Initialize total and index to zero; test `index < count` before acquisition.
2. Construct `a.b(index)` and call its `a()` method.
3. Add the read result, close the resource, then increment index.
4. On a read failure E, attempt close and rethrow E; if close fails with C,
   call `E.addSuppressed(C)` first, assuming distinct identities and a normally
   returning `addSuppressed`.
5. Propagate a normal-close failure without closing again or advancing the loop.

This matches the unobfuscated Java resource loop, including no acquisition for
nonpositive count and the expected result 6 for count 3. The helper's renamed
fields still store the id and closed flag; its read computes twice the id and
its close writes the flag.

The concrete close does not deliberately throw, and the freshly acquired object
is read before close. Failure paths above were compared by reading their control
and catch structure, not by executing injected faults. Automated behavioral
execution has not been performed.

### Readability and identities

The cleanup handler and its close/suppression catch now belong to the loop body,
without a detached handler or explicit exception chain in the resource method.
The same recovery works for the original and obfuscated member identities.

ProGuard flattened the nested classes to `a.a` and `a.b` and removed
`@MemberClasses`. Missing input attributes cannot be reconstructed from a name.
The three `a` overloads still require their full parameter signatures for review.

### Ranked fixes

Improve local readability without replacing original obfuscated member identities
or inventing missing class metadata.

## Rounds

- R3: Direct source/output reading, paired by original signature, finds resource
  behavior consistent. This is a manual judgment, not an executed behavioral test.
- R2: The obfuscated resource loop recovered nested protection alongside the
  original fixture; its manual semantic judgment was left pending.
- R1, 2026-11-08: class flattening, removed member-class metadata
  and overloaded short names complicated review.
