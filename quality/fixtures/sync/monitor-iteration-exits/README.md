# Monitor exits to an iteration continuation

## Independent expectations

Authored Java/Smali, no third-party input. Initial discovery:
`../../loops/iteration-tail-cleanup/` locked. The initial repair completes a
specialized release Follow target against the actual selected enclosing loop,
without changing release/routing/copy facts. This new challenge adds typed
recovery, held finally cleanup, separate normal/continue/break/return releases
and exceptional replacement. Its guard keeps a monitor-protected backedge that
deliberately skips the update.

Owner: `Lfixture/MonitorIteration;`. Subjects:
`heldCleanup(Ljava/lang/Object;II)I`, `walk(Ljava/lang/Object;II)I`,
`bypass(Ljava/lang/Object;II)I`.
All read/cleanup/mark bodies are in the DEX. trace starts at zero. Constructors
succeed; asynchronous/linkage/allocation faults and concurrent trace writers are
outside the model. Java's implicit constructor is not a Smali subject.

walk acquires before read and performs cleanup while held. NFE/NPE mark 20 and
select 100; broader RuntimeException marks 21 and selects -2. Every normal path
releases before any counter update, loop exit or return. Error and cleanup
failure release before propagation; cleanup mode 6 replaces a pending Error.

Notation: L=acquire, U=release, R(i)=mark(10+i), C(i)=mark(30+i).

| Subject | Arguments | Expected exit | Ordered effects |
|---|---|---|---|
| heldCleanup | (lock,3,0) | return 4 | For i=0..2: L,R(i),C(i),U; value2 skips addition, not update |
| heldCleanup | (lock,3,1) or (lock,3,2) | return 300 | For i=0..2: L,R(i),mark20,C(i),U |
| heldCleanup | (lock,3,3) | return -6 | For i=0..2: L,R(i),mark21,C(i),U |
| heldCleanup | (lock,3,4) | throw original Error | L,R(0),C(0),U |
| heldCleanup | (lock,3,5) or (lock,3,6) | throw IAE from cleanup | L,R(0),C(0),U; no update; mode6 replaces Error |
| heldCleanup | (null,1,0) | throw NPE at acquisition | none |
| heldCleanup | (null,0,6) | return 0 | none |
| walk | (lock,3,0) | return 4 | For i=0..2: L,R(i),C(i),U; value2 skips addition but still updates |
| walk | (lock,3,1) or (lock,3,2) | return 200 | For i=0..1: L,R(i),mark20,C(i),U; break skips final update |
| walk | (lock,3,3) | return -2 | L,R(0),mark21,C(0),U; no update |
| walk | (lock,3,4) | throw original Error | L,R(0),C(0),U |
| walk | (lock,3,5) | throw IAE from normal cleanup | L,R(0),C(0),U; no addition/update |
| walk | (lock,3,6) | throw IAE replacing Error | L,R(0),C(0),U; no suppression |
| walk | (null,1,0) | throw NPE at acquisition | none |
| walk | (null,0,6) | return 0 | none |
| bypass | (lock,3,0) | return 0 | L,mark50,U,L,mark51,U,L,mark52,U |
| bypass | (lock,3,1) | return 2 | L,mark50,U,L,mark50,U; row stays zero |
| bypass | (null,1,0) | throw NPE at acquisition | none |

heldCleanup is the fresh same-mechanism challenge: all normal releases share
the same original step, including value2's continue, while recovery and finally
work remain held. Its available natural scope must retain the loop's supplied
continue role rather than a goto to a hidden header update. It adds the helper
and cleanup interactions absent from the initial locked seed. These expectations
for this additional subject precede its first candidate inspection.

bypass has two original backedges. Only the normal advance path increments row;
the flagged continuation releases then retests without increment. Completing
its release role must not invent a common for update. Returning seen=2 also
releases before return, with no pending update.

The release Kind may gain a selected-loop source role, but its original target,
routing and before/after copy phases must remain the same. A natural closing
brace does not add another implicit release. These expectations were recorded
before candidate inspection. No behavioral execution is claimed.

## Preparation

```sh
mkdir -p .cache/fixtures/monitor-iteration-exits
java -cp '.cache/evolution-demo/smali-tools/*' org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/monitor-iteration-exits/classes.dex \
  quality/fixtures/sync/monitor-iteration-exits
cp .cache/fixtures/monitor-iteration-exits/classes.dex \
  quality/fixtures/sync/monitor-iteration-exits/classes.dex
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/sync/monitor-iteration-exits/classes.dex \
  --exec '{"cmd":"p6-all","details":true}' \
  --exec '{"cmd":"p9-all","text_only":true}'
```

Prepared with Smali 2.5.2, API 24 and OpenJDK 21.0.12.1. Baseline is the recorded
working tree on `ebbc003` after ordinary iteration-tail recovery and before
selected monitor continuation completion. Comparisons use the same prepared DEX.
The new heldCleanup case retains its natural scope and supplied for continuation.
walk deliberately keeps raw monitors where early source selection has incompatible
value-return/step follows; bypass retains while. An initial prior-fixture dangling
goto regression was found and repaired in the shared P7 consumer before the final
reading judgment. See review.md for scope and unrun behavioral execution.
