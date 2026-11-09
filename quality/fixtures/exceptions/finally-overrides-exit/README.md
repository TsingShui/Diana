# Finally overrides a pending exit

## Input and expectations

The Java explanation and Smali input are independently authored for this
repository; they contain no third-party code or data. Original owner:
`Lfixture/FinallyOverridesExit;`. The subject is
`Lfixture/FinallyOverridesExit;->replaceExit(I)I`.

The method selects body and finally behavior with mode bits: bit `1` throws
`IllegalStateException("body")` before the read, bit `2` returns `-7` from the
finally, and bit `4` throws `IllegalArgumentException("finally")` from the
finally. The normal body calls `read(res)` and returns `5`; the finally first
calls `close(res)`.

`open(I)I`, `read(I)I` and `close(I)V` are native declarations. Each scenario
starts with an empty trace. Unless a scenario states a fault, `open(0) = 7`,
`read(7) = 70`, and `close(7)` returns normally. A selected `open`, `read` or
`close` call may instead throw. Fault throwables are distinct and non-null: `E`
for a helper failure, `C` for a close failure, and the constructed `ISE` and
`IAE` for the body and finally throws. No suppression call occurs in this
fixture. No asynchronous exception or allocation failure is assumed.

Notation: `O`, `R`, `C` denote open, read and close. A return or throw in the
finally replaces the pending body return or exception rather than being combined
with it. The ordered-call column records helper calls only; the constructed
throwable is named by type and message.

| Subject and arguments | Fault or selected exit | Expected exit | Ordered calls and effects |
|---|---|---|---|
| `replaceExit(0)` | normal body | return 5 | O(0), R(7), C(7) |
| `replaceExit(1)` | body ISE | throw ISE("body") | O(0), C(7) |
| `replaceExit(2)` | finally return | return -7 | O(0), R(7), C(7) |
| `replaceExit(4)` | finally throw | throw IAE("finally") | O(0), R(7), C(7) |
| `replaceExit(6)` | bit 2 before bit 4 | return -7 | O(0), R(7), C(7) |
| `replaceExit(5)` | body ISE and finally IAE | throw IAE("finally"), ISE discarded | O(0), C(7) |
| `replaceExit(3)` | body ISE and finally return | return -7, ISE discarded | O(0), C(7) |
| `replaceExit(0)` | C(7) throws C | throw C, return 5 discarded | O(0), R(7), C(7) |
| `replaceExit(1)` | body ISE; C(7) throws C | throw C, ISE discarded without suppression | O(0), C(7) |
| `replaceExit(2)` | C(7) throws C | throw C, return -7 not reached | O(0), R(7), C(7) |
| `replaceExit(0)` | R(7) throws E | throw E | O(0), R(7), C(7) |
| `replaceExit(2)` | R(7) throws E; C(7) normal | return -7, E discarded | O(0), R(7), C(7) |
| `replaceExit(4)` | R(7) throws E; C(7) normal | throw IAE("finally"), E discarded | O(0), R(7), C(7) |
| `replaceExit(4)` | R(7) throws E; C(7) throws C | throw C, E replaced; IAE construction not reached | O(0), R(7), C(7) |

The read-failure/finally-return scenario uses mode 2: bit 1 is clear, so the read
is reached, and bit 2 selects the finally return. Mode 3 instead throws the body
ISE before any read; that separate scenario is retained above. These corrections
follow the Java/Smali control flow, not observed candidate results.

## Trigger structure

The Smali has a normal-path copy of the finally after the protected body and a
catch-all handler copy after `move-exception`. The protected range covers the mode
checks, the body throw, the read and the return operand; it excludes the normal
close, the return/throw overrides and the handler's own close. A close failure
therefore leaves the protected range and replaces the pending exit. A return in
the finally is checked before its throw. The prepared DEX is assembled from
`FinallyOverridesExit.smali`. The historical baseline retained explicit cleanup;
the current abrupt-cleanup repair selects one natural finally from the existing
P5 construction and P6 protection facts.

## Preparation and inspection

```sh
mkdir -p .cache/fixtures/finally-overrides-exit
java -cp '.cache/evolution-demo/smali-tools/*' org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/finally-overrides-exit/classes.dex \
  quality/fixtures/exceptions/finally-overrides-exit
cp .cache/fixtures/finally-overrides-exit/classes.dex \
  quality/fixtures/exceptions/finally-overrides-exit/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/finally-overrides-exit/classes.dex \
  --exec '{"cmd":"p6-all","details":true}' \
  --exec '{"cmd":"p9-all","text_only":true}'
```

Prepared with Smali 2.5.2, `--api 24`, on OpenJDK 21.0.12.1. The current
confirmation used repository HEAD `6d663d9`; engine sources are unchanged from
`12de370`. `review.md` records the reading judgment.

## Review status

The current review is primary LLM source/output path reading. Six grouped
ordinary-Engine abrupt-finally tests cover this original case and its fresh
[complexity challenge](../abrupt-finally-completion/README.md); all passed.
No complete-method behavioral executor or Java recompilation was run. The helper failures are
scenario inputs; the constructed exception identities come from the input's
`new-instance` and constructor calls.
