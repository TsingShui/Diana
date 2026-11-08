# Finally return-value capture

## Input and expectations

The Java explanation and Smali input are independently authored for this
repository; they contain no third-party code or data. Original owner:
`Lfixture/FinallyReturnCapture;`. The subjects are
`Lfixture/FinallyReturnCapture;->localReturn(I)I` and
`Lfixture/FinallyReturnCapture;->fieldReturn(I)I`.

`record(I)V` is a native helper. The fixture has one static field, `state`.
Java evaluates the return operand before entering the finally; the finally then
overwrites the local or field and records the new value. The returned value must
remain the value evaluated before the overwrite.

Each scenario starts with an empty trace. `record(value)` records its argument and
returns normally unless a scenario states that it throws `A`. No asynchronous
exception or allocation failure is assumed. `localReturn` starts with
`result = 1`; mode 1 changes it to 2 before the return, and mode 2 throws
`ISE("body")` before the return. `fieldReturn` starts with `state = 10`; mode 1
throws `ISE("body")` before reading the field, and the finally sets `state = 99`.

Notation: `REC` denotes a record call. The finally executes after the return
operand is evaluated and before the return or rethrow completes.

| Subject and arguments | Fault | Expected exit | Ordered calls and effects |
|---|---|---|---|
| `localReturn(0)` | none | return 1 | REC(3) |
| `localReturn(1)` | none | return 2 | REC(3) |
| `localReturn(2)` | body ISE | throw ISE("body") | REC(3) |
| `localReturn(0)` | REC throws A | throw A, return 1 discarded | REC(3) |
| `localReturn(2)` | REC throws A | throw A, ISE discarded | REC(3) |
| `fieldReturn(0)` | none | return 10; field 99 | REC(99) |
| `fieldReturn(1)` | body ISE | throw ISE("body"); field 99 | REC(99) |
| `fieldReturn(0)` | REC throws A | throw A; field 99 | REC(99) |
| `fieldReturn(1)` | REC throws A | throw A; field 99 | REC(99) |

`localReturn(0)` must not return 3 and `fieldReturn(0)` must not return 99: those
are the values after the finally overwrite. The record call observes the
overwritten value in both cases. A record failure is an abrupt finally completion
and replaces the pending return or body exception.

## Trigger structure

The Smali stores the pending return value in a separate register before the
finally copy. The normal-path finally copy writes the new result or field, calls
`record`, and returns the saved register. The catch-all handler captures the
throwable, performs the same finally writes and record call, and rethrows.

The prepared DEX is assembled from `FinallyReturnCapture.smali`. The p6 product
reports no natural try item and two chains (one handler chain plus the uncaught
chain) for each method: the protected
body and the normal-path finally are explicit. The `fieldReturn` output reads the
field into a local before the write; the local and field share the printed name
`state`.

## Preparation and inspection

```sh
mkdir -p .cache/fixtures/finally-return-capture
java -cp '.cache/evolution-demo/smali-tools/*' org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/finally-return-capture/classes.dex \
  quality/fixtures/exceptions/finally-return-capture
cp .cache/fixtures/finally-return-capture/classes.dex \
  quality/fixtures/exceptions/finally-return-capture/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/finally-return-capture/classes.dex \
  --exec '{"cmd":"p6-all","details":true}' \
  --exec '{"cmd":"p9-all","text_only":true}'
```

Prepared with Smali 2.5.2, `--api 24`, on OpenJDK 21.0.12.1. The current
confirmation used repository HEAD `6d663d9`; engine sources unchanged from
`12de370`. `review.md` records the reading judgment.

## Review status

The review is a manual source/output reading judgment. No automated behavioral
executor, Java recompilation or fixture test was run. `record` is a native helper;
its argument and failure behavior are scenario inputs.
