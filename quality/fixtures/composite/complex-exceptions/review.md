# Fixture: complex-exceptions

Nested finally, caught rethrow with a cause, resources inside a loop and
multi-catch. `ComplexExceptions.java` is the authored reference; the committed
DEX was prepared with `javac --release 8 -g` and `d8 --min-api 24` through
`quality/build.sh`.

- Provenance: authored for this repository; no third-party code or data.
- Regenerate from the sources; never edit the DEX by hand.

## Current verdict — round 3

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
