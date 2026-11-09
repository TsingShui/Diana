# Fixture: monitor-iteration-exits

## Current verdict — split-outlet review

Java/Smali, helper scenarios and final walk/bypass were reread. Closed scopes now
retain distinct explicit outlets with no shared normal follow. walk's negative
path releases then jumps to the original value return outside the for; value2
releases then continues through the update; total>=200 releases then breaks;
ordinary success releases then updates. Exceptional cleanup/replacement still
releases and throws without updating. The recorded results 4,200,-2 and throw
identities/order are unchanged. No value-return dispatcher was invented.

bypass now also has a natural scope, but still a while with two backedges. Its
flagged continuation releases then retests at row0; mark50 twice returns seen=2
after its separate release/return outlet. The normal flag0 branch releases then
jumps to its visible ordinary row update. It was not retargeted to that update
merely because a synchronized scope became available. heldCleanup and helper
text are unchanged and reuse the prior review.

This is bounded LLM path reading, not executed behavioral PASS. The new
[split-tail challenge](../monitor-split-tails/review.md) covers nested captures,
three unlocked targets and a real executable post-release Phi-copy barrier.
Original P5 plans and outlet phases are unchanged; selected synchronized ownership
removes only duplicate ordinary link-copy references, with one original copy
anchor retained. No device run or Java recompilation ran.

## Prior LLM semantic review

Compared Java, actual Smali, concrete helpers and final ordinary Engine output.
Independent scenarios precede candidate inspection; heldCleanup was added with
its own expectations before that subject's first inspection. The same DEX is used
on the recorded pre-monitor working tree based on `ebbc003` and the candidate.
No Java recompilation, device run or behavioral scenario executor ran.

| Original method, owner Lfixture/MonitorIteration; | Judgment |
|---|---|
| `heldCleanup(Ljava/lang/Object;II)I` | Equivalent on the recorded paths; the natural held scope releases before its supplied update continuation. |
| `walk(Ljava/lang/Object;II)I` | Equivalent with original explicit monitors; continue, break, return and exceptional replacement retain separate releases. |
| `bypass(Ljava/lang/Object;II)I` | Equivalent ordinary while; its released header backedge still skips row++. |
| `read(II)I`, `cleanup(II)V`, `mark(I)V` | Same original events, arguments, result/exception choices and wrapping trace arithmetic. |

### Held normal and recovered paths

Each row acquires before read and cleans while held. Value 2 skips addition, not
the update: normal rows 0..2 select 1,2,3, returning 4. Both original normal
releases reach the same original update. They now carry the actual selected for
loop's Continue role, so no goto targets an update hidden in its header. Each
successful release still precedes one update, then the original head test.

NFE/NPE share one handler, mark 20 and select 100; three rows return 300. The
broader RuntimeException catch marks 21 and selects -2; three rows return -6.
Neither recovery leaves the lock before cleanup or repeats cleanup after release.

Error bypasses typed handlers. Its captured value survives successful cleanup
and is rethrown into the original monitor handler; that handler releases and
throws the same pending object. Mode5 cleanup fails before addition/update; mode6
cleanup replaces the Error. The IAE releases then propagates without suppression
or retry. Null acquisition executes no read/cleanup/release, and zero rows skip
the acquisition entirely. The closing synchronized brace is not another release.

### Mixed exits and negative guard

walk's normal path likewise returns 4. Modes1/2 add 100 twice, then branch to the
original outside-for release and return 200, with no final row increment. Mode3
cleans, branches to its distinct release and returns -2 without addition/update.
Modes4/5/6 retain the same exceptional release/identity/replacement as heldCleanup.
All six original monitor operations retain one nonempty source anchor.

This method still has explicit monitor instructions: the existing early source
scope selection sees incompatible value-return/step follows. The repair does not
reopen that scope or invent a value-return protocol. The for update is independently
available from the ordinary loop product; the raw break/return labels still bypass
it correctly. A fully natural multi-outlet monitor is not claimed.

bypass remains while with two original backedges. Flag1 marks 50 twice and
returns seen=2 after its own release, never incrementing row. Flag0 marks 50,51,52
and returns zero, releasing before each ordinary update. No monitor exit authorizes
replacing the direct header backedge with the step path.

## Owning repair and prior-effect regression

P6 finalizes a retained Follow(update) only from an actual selected enclosing
Loop with that supplied iteration target. It preserves release/link/target,
routing and before/after copies. Existing incompatible Loop intents are not
reinterpreted; missing selected context keeps Follow. Early lock propagation,
shared-follow selection and unavailable scope reasons are not retried.

An initial candidate passed 38 structural checks but regressed the prior nested
fixture: NestedCatchLoops.run printed goto L13 before a new continue even though
L13's update had moved to its header. Direct LLM reading caught the dangling
destination; initial effectiveness was blocked. Root cause was P7's ReleasedSync
initializer, which emitted an ordinary goto before the control continuation P6
already supplied. P7 now lowers this syntax-owned transfer to Fallthrough, retaining
original copies/routing and executing the supplied control completion once.
A new direct-release regression check catches that producer/consumer disagreement.

Final readings cover all six changed fixture texts. NestedCatchLoops now retains
both updates, held cleanup/release and foreign-entry behavior. The seed locked
subject releases before update. SyncBlock's nested/return/void paths retain their
lock order without redundant gotos. SyncCleanupRethrow still closes normally after
release, exceptionally while held; SyncFinallyReturn still closes/records before
release and retains its planned goto past the exceptional handler.

Within these reviewed paths the repair is effective: the new held-cleanup
challenge exercises the supplied release continuation through typed recovery and
replacement, the bypass guard remains ordinary, and the initial regression is
repaired. This is not universal verification or authorization to commit.

## Supporting checks and limitations

Four monitor-continuation package checks consume ordinary Engine delivery: held
outlet targets, direct-release control completion, original lock anchors for mixed
exits and the released update-skipping backedge. The earlier locked check now
consumes its delivered Continue fact instead of requiring an obsolete while
qualification. No checker executes the scenarios above.

All-feature nextest passed 39 package checks after the final consumer repair.
Corpus inspection covers 80 fixtures, 131 classes and 612 methods with no stopped
member, render failure or render issue. Exact P2/P3/P4/P5 and relevant original P6
control/protection/lock facts agree; synchronized targets/routing/before/after plans
also agree exactly apart from supplied source-role Kind. Other fixture text is
unchanged from the recorded pre-monitor tree and reuses prior reviews.

Default and all-feature nextest each passed 39 checks. Package Clippy with
warnings denied, release and dbg_shell builds, format/diff and desensitization
checks passed. Execution remains unrun; asynchronous/linkage/allocation and
concurrent effects outside the README assumptions remain outside this judgment.
