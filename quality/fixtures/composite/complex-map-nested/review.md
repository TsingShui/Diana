# Fixture: complex-map-nested

nested generics, computeIfAbsent with a lambda, getOrDefault and entry loops. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — round 1, 2026-11-08

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
