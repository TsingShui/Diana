# Fixture: algo-string

KMP prefix/search, word frequency over HashMap and string reversal. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — iteration-tail review

Java and final text were reread for indexOf(String,String) and topWords(String).
indexOf advances only after its fallback, character comparison and match-length
test; an early successful return skips the update. Empty pattern still returns
before touching text. topWords advances after its optional nonempty-word
get/unbox/count/put, including the empty-word skip path. Exceptions skip updates.
The captured array length, regex units, later iterator traversal, prefix and
reverse remain unchanged.

This is direct LLM source/path evidence on the candidate based on `ebbc003`, not
executed behavioral PASS. Original P2/P3/P4/P5 and relevant P6 control/exception/
lock facts agree in corpus inspection. Unchanged member text reuses prior reviews.
The new [iteration-tail challenge](../../loops/iteration-tail-cleanup/review.md)
records the interacting cases and unsupported execution scope.

## Prior verdict — round 1, 2026-11-08

### Semantics

The verdict is written by reading the reference next to the output.

| method | semantics |
|---|---|
| `prefix` | ✔ Prefix-table recurrence agrees. |
| `indexOf` | ✔ Mismatch fallback and match positions agree. |
| `topWords` | ✔ Counting and filtering agree. |
| `reverse` | ✔ Reverse concatenation agrees, but StringBuilder allocation and constructor invocation remain split. |

### Findings

- `topWords` reads Map.Entry keys/values through Object and casts; `via Map$Entry` is visible. Recorded forms: `getValue`, `getKey`.
- `reverse` prints allocation followed by `stringBuilder.<init>(v2)` instead of one constructor expression. Recorded forms: `new StringBuilder;`.
- HashMap/ArrayList use raw types; entry iteration exposes `Iterator v15 = hashMap.entrySet().iterator()`.

### Ranked fixes

Improve constructor composition and generic arguments; reduce Object intermediates and explicit owner notation. Recorded forms: `new X(...)`, `via`.

## Rounds

- R1 2026-11-08: `topWords` reads Map.Entry keys/values through Object and casts; `via Map$Entry` is visible. Recorded forms: `getValue`, `getKey`.
