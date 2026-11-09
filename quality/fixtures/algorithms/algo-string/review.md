# Fixture: algo-string

## Current verdict — scalar-use disclosure

On the same prepared DEX, baseline df10b74 versus this change differs only at
reverse's character append. Its int carrier now states the supplied char view;
the original charAt, append(C), decrement and exit order remain. Empty input
performs no append; a reached access failure skips append/update. Under the
ordinary StringBuilder model, reverse("ab") yields "ba", not the decimal codes.
All P2–P8 products are exact in this comparison. This is bounded LLM source/path
reading, not execution or a claim about arbitrary library/allocation behavior.

### Semantics

| Method | Judgment and scope |
|---|---|
| `reverse` | ~ The new `(value via char)` preserves the original UTF-16 unit and char binding at each append, without conversion or another read. |
| `prefix` | ~ Unchanged text/products; prior recurrence and character-access judgment is reused. |
| `indexOf` | ~ Unchanged text/products; prior empty-pattern and successful-match guard judgment is reused. |
| `topWords` | ~ Unchanged text/products; prior selected counting/iterator-path judgment is reused. |

The [scalar challenge](../../exceptions/scalar-views-protected-joins/review.md)
adds joins, copies, loop recovery, cleanup and composition. Historical execution
claims below were not rerun and do not certify this output.


## Prior verdict — while/do-while architecture correction

Read the early KMP match as well as normal scanning: match returns before the
index update; only a nonmatch advances. Prefix/reverse/word loops retain their
access/call order and body-end updates.

Compared the same prepared DEX on 8ac6d95 and this commit. Dedicated for recovery
contradicted the approved architecture and is removed, not retained behind a
switch. Updates stay in ordinary body positions with original targets/copies;
explicit gotos can be necessary instead of update-skipping while continues.
The [fresh challenge](../../loops/while-tail-continuations/review.md) covers
three-level private continuations, real shared entrances, cleanup and do-while.
This is bounded LLM source/path reading, not behavioral execution. Historical
execution claims below were not rerun and do not certify this output.


## Prior verdict — whole-operation follow-up

Read the changed string/index loops. Character accesses, tests and selected
accumulator work retain their order; index tails remain after the completed
body. Zero-length/early-exit paths do not acquire another access or update. No
external String implementation facts are used by recovery.

The [pure-tail challenge](../../exceptions/pure-tail-finally/review.md) and
[carrier challenge](../../atoms/refined-carriers/review.md) record the owning
producer/consumer repairs and blocked drafts. This is bounded current source
reading; historical execution claims below were not rerun and do not certify
this output. No behavioral executor, device run or Java recompilation ran.


KMP prefix/search, word frequency over HashMap and string reversal. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Prior verdict — iteration-tail review

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

- Scalar-use disclosure: reread reverse's original char append and retained its access/update order; other member text is unchanged.
- R1 2026-11-08: `topWords` reads Map.Entry keys/values through Object and casts; `via Map$Entry` is visible. Recorded forms: `getValue`, `getKey`.
