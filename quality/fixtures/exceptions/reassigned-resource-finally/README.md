# Reassigned resource with a conditional finally

## Input and expectations

The Java explanation and Smali input are independently authored for this
repository; they contain no third-party code or data. Original owner:
`Lfixture/ReassignedResourceFinally;`. The subject is
`Lfixture/ReassignedResourceFinally;->rotate(I)I`.

`read(I)I`, `open(I)I` and `close(I)V` are native helpers. The method reads an
acquisition argument, opens a resource, records an `acquired` flag, reads the
resource, optionally closes it and reassigns the slot to a second acquisition,
and uses a conditional finally to close whichever handle is still owned.

Each scenario starts with an empty trace. Unless a scenario states a fault,
`read(0) = 5`, `open(5) = 50`, `read(50) = 500`, `open(6) = 60`,
`read(60) = 600`, and every close returns normally. Mode 1 selects the close and
reassignment branch. A scenario may make a selected helper call throw. Fault
throwables are distinct and non-null: `E` for a selected read/open failure and
`C` for a selected close failure. The explicit finally replaces a pending
exception when its conditional close fails; no suppression call is used.
No asynchronous exception or allocation failure is assumed.

Notation: `R`, `O`, `C` denote read, open and close. The flag is set before the
old close and after the new acquisition. The expected calls list records every
helper call, including a close that throws.

| Subject and arguments | Fault | Expected exit | Ordered calls and effects |
|---|---|---|---|
| `rotate(0)` | none | return 500 | R(0), O(5), R(50), C(50) |
| `rotate(1)` | none | return 1100 | R(0), O(5), R(50), C(50), O(6), R(60), C(60) |
| `rotate(0)` | R(0) throws E | throw E | R(0) |
| `rotate(0)` | O(5) throws E | throw E | R(0), O(5) |
| `rotate(0)` | R(50) throws E | throw E | R(0), O(5), R(50), C(50) |
| `rotate(0)` | R(50) throws E; C(50) throws C | throw C | R(0), O(5), R(50), C(50) |
| `rotate(0)` | C(50) throws C | throw C | R(0), O(5), R(50), C(50) |
| `rotate(1)` | R(50) throws E | throw E | R(0), O(5), R(50), C(50) |
| `rotate(1)` | C(50) throws C | throw C | R(0), O(5), R(50), C(50) |
| `rotate(1)` | O(6) throws E | throw E | R(0), O(5), R(50), C(50), O(6) |
| `rotate(1)` | R(60) throws E | throw E | R(0), O(5), R(50), C(50), O(6), R(60), C(60) |
| `rotate(1)` | R(60) throws E; C(60) throws C | throw C | R(0), O(5), R(50), C(50), O(6), R(60), C(60) |
| `rotate(1)` | C(60) throws C | throw C | R(0), O(5), R(50), C(50), O(6), R(60), C(60) |

The important paths are the two boundaries: a failure of the old close happens
after the flag is cleared, so the finally must not close the old handle again; a
failure of the second acquisition leaves the old handle already closed and the
flag clear; a failure of the second read leaves the new handle owned and the
finally closes it. The first acquisition and the seed read are outside the try,
so their failures never enter the finally.

## Trigger structure

The protected region begins after the first acquisition. It contains the first
read, the flag clear, the old close, the second acquisition from `seed + 1`, the
flag set and the second read. The normal-path finally and the catch-all handler
both test the flag and conditionally call `close(current)`. The close calls and
the whole finally tail are outside the protected range, so a close failure
replaces the pending exit.

The prepared DEX is assembled from `ReassignedResourceFinally.smali`. The p6
product reports one natural try item and two chains (one handler chain plus the
uncaught chain) for `rotate`. The
handler receives the flag and current-handle snapshots from the throwing
operation.

## Preparation and inspection

```sh
mkdir -p .cache/fixtures/reassigned-resource-finally
java -cp '.cache/evolution-demo/smali-tools/*' org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/reassigned-resource-finally/classes.dex \
  quality/fixtures/exceptions/reassigned-resource-finally
cp .cache/fixtures/reassigned-resource-finally/classes.dex \
  quality/fixtures/exceptions/reassigned-resource-finally/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/reassigned-resource-finally/classes.dex \
  --exec '{"cmd":"p6-all","details":true}' \
  --exec '{"cmd":"p9-all","text_only":true}'
```

Prepared with Smali 2.5.2, `--api 24`, on OpenJDK 21.0.12.1. The current
confirmation used repository HEAD `6d663d9`; engine sources are unchanged from
`12de370`. `review.md` records the reading judgment.

## Review status

The review is a manual source/output reading judgment. No automated behavioral
executor, Java recompilation or fixture test was run. The helper bodies are native;
the scenario table supplies their results and faults.
