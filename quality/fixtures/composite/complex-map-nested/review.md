# Fixture: complex-map-nested

## Current verdict — terminal guard follow-up

Read the null groups lookup guard. Null returns0 before iterator creation; a
present list retains casts, calls, accumulator and visible while update. The
initial lookup still occurs once before the guard.

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

Read the changed nested traversal/update paths. Receiver loads, map/entry calls,
selected copies and mutations retain their order through inner and outer
iterations. Pure scalar tails still execute at their original body ends, not
before entry tests or after an abrupt exit.

The [pure-tail challenge](../../exceptions/pure-tail-finally/review.md) and
[carrier challenge](../../atoms/refined-carriers/review.md) record the owning
producer/consumer repairs and blocked drafts. This is bounded current source
reading; historical execution claims below were not rerun and do not certify
this output. No behavioral executor, device run or Java recompilation ran.


nested generics, computeIfAbsent with a lambda, getOrDefault and entry loops. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Prior verdict — round 1, 2026-11-08

### Semantics

The verdict is written by reading the reference next to the output.

| method | semantics |
|---|---|
| `add` | ✔ computeIfAbsent and its lambda class agree. |
| `total` | ✔ Null selection and summation agree. |
| `totals` | ✔ Nested iteration and write-back agree. |
| `countOrZero` | ✔ `getOrDefault(key, new ArrayList<>())` |
| `merge` | ✔ computeIfAbsent and addAll agree. |

### Findings

- The nested Map<String, List<Integer>> declaration becomes raw Map groups, with casts on retrieved elements.
- Explicit owner notation and the standalone `((String) key);` check-cast statement appear in the body. Recorded forms: `via Map$Entry`, `((String) key); // Diana: check-cast, value not reassigned`.
- Enhanced entrySet iteration expands into Iterator and Object values. Recorded forms: `entrySet()`.

### Ranked fixes

Improve nested generic and entry-iteration presentation while preserving actual checks and owners. Recorded forms: `via`, `for (Map.Entry<…> e : …)`.

## Rounds

- R1 2026-11-08: The nested Map<String, List<Integer>> declaration becomes raw Map groups, with casts on retrieved elements.
