# Labelled exits through resource cleanup

## Input and expectations

The Java explanation and Smali input are independently authored for this
repository; they contain no third-party code or data. Original owner:
`Lfixture/LabelledExitCleanup;`. The subject is
`Lfixture/LabelledExitCleanup;->scan(II)I`.

`open(I)I`, `read(I)I` and `close(I)V` are native helpers. Java uses a small
`Res` wrapper around a handle so that the source can show try-with-resources;
Smali executes the same close with an integer handle. Diana receives no wrapper
declaration or helper body.

Each scenario starts with an empty trace. Unless a scenario states a fault,
`open(row) = row`, `read(handle) = 10 * handle + 1`, and `close(handle)` returns
normally. A scenario may override a selected read value or make a selected
`open`, `read` or `close` call throw. Fault throwables are distinct and non-null:
`E` for the selected primary call and `C` for a close failure.
`Throwable.addSuppressed` records its receiver and argument and returns normally.
No asynchronous exception or allocation failure is assumed.

Notation: `O`, `R`, `C` denote open, read and close; `S(E,C)` denotes
`E.addSuppressed(C)`. The action column names the source path selected by the
scenario. `limit` is the second argument.

| Subject and arguments | Fault or selection | Expected exit | Ordered calls and effects |
|---|---|---|---|
| `scan(2, 999)` | normal inner completion | return 48 | O(0), R(0) ×4, C(0), O(1), R(1) ×4, C(1) |
| `scan(2, 11)` | first read in row 1 equals limit | return 4 | O(0), R(0) ×4, C(0), O(1), R(1), C(1) |
| `scan(3, 999)` | total exceeds 100 in row 2 | return 111 | O(0), R(0) ×4, C(0), O(1), R(1) ×4, C(1), O(2), R(2) ×3, C(2) |
| `scan(2, 999)` | first read of row 0 returns -5 | return 0 | O(0), R(0), C(0) |
| `scan(0, 0)` | empty outer loop | return 0 | none |
| `scan(2, 999)` | second R(0) throws E; C(0) throws C | throw E, suppressed [C] | O(0), R(0), R(0), C(0), S(E,C) |
| `scan(2, 999)` | second R(0) throws E | throw E | O(0), R(0), R(0), C(0) |
| `scan(2, 999)` | normal C(0) throws C | throw C | O(0), R(0) ×4, C(0) |
| `scan(2, 11)` | continue path; C(1) throws C | throw C | O(0), R(0) ×4, C(0), O(1), R(1), C(1) |
| `scan(2, 999)` | first read returns -5; C(0) throws C | throw C | O(0), R(0), C(0) |
| `scan(3, 999)` | return path; C(2) throws C | throw C | O(0), R(0) ×4, C(0), O(1), R(1) ×4, C(1), O(2), R(2) ×3, C(2) |
| `scan(2, 999)` | O(1) throws E | throw E | O(0), R(0) ×4, C(0), O(1) |

The first three rows use `read(0) = 1` and `read(1) = 11`. The normal
inner-completion close is followed by the outer increment. A labelled continue
closes the handle before incrementing the row; a labelled break closes it and
leaves the outer loop without the increment; a return closes it before returning.
A close failure on any of those normal exits discards the pending transfer and
propagates.

## Trigger structure

The handle is acquired in the outer loop body. The inner loop has four possible
normal exits, and the Smali protects only the read and the following non-throwing
comparison and update instructions. Every exit-specific close and the normal
inner close are outside the catch-all range, so a close failure cannot re-enter
the resource handler. The handler closes the handle, suppresses a close failure
on the primary and rethrows. The suppression handler protects only its own close
call.

The prepared DEX is assembled from `LabelledExitCleanup.smali`. The p6 product
reports two try items and two handler chains besides the uncaught chain for
`scan`: the read protection and
the close/suppression protection.

## Preparation and inspection

```sh
mkdir -p .cache/fixtures/labelled-exit-cleanup
java -cp '.cache/evolution-demo/smali-tools/*' org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/labelled-exit-cleanup/classes.dex \
  quality/fixtures/exceptions/labelled-exit-cleanup
cp .cache/fixtures/labelled-exit-cleanup/classes.dex \
  quality/fixtures/exceptions/labelled-exit-cleanup/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/labelled-exit-cleanup/classes.dex \
  --exec '{"cmd":"p6-all","details":true}' \
  --exec '{"cmd":"p9-all","text_only":true}'
```

Prepared with Smali 2.5.2, `--api 24`, on OpenJDK 21.0.12.1. The current
confirmation used repository HEAD `6d663d9`; engine sources are unchanged from
`12de370`. `review.md` records the reading judgment.

## Review status

The review is a manual source/output reading judgment. No automated behavioral
executor, Java recompilation or fixture test was run. The native helper behavior
is modeled by the scenario table; a close that fails is a selectable scenario,
not a fault hard-coded in the helper declaration.
