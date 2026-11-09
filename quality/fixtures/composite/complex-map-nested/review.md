# Fixture: complex-map-nested

## Current verdict — whole-operation follow-up

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
