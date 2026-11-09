# Fixture: abrupt-continuation-tails

## Current verdict — protected-entry follow-up

Java/Smali and final decorated(III)I were reread. The selected read now has its
natural catch-all instead of an explicit site-chain dispatch. Its optional body
prefix includes the pure second-switch selection/stores, not the first-stage
matches calls. Those nonthrowing instructions keep their original order before
the read and introduce no new throwing event. The first-stage shared non-case
tail remains explicit; its completed-break regression is unchanged.

Normal selection still reads10/20/30 or-1, cleans90 and marks91 before return.
A read failure still captures the same ISE, marks90 and throws without mark91.
The result declaration moves outside the try because it is read later, but its
assignment still occurs only after successful read; no default value is invented.
Other members are unchanged. The new
[prefix-frontier challenge](../../exceptions/protected-prefix-frontiers/review.md)
records the owning entry fact and a genuine multi-entry guard.
This is bounded LLM reading, not executed behavioral PASS. No device run or Java
recompilation ran.

## Prior LLM semantic review

Compared Java/Smali, concrete helper bodies and final ordinary Engine output on
the same DEX. Independent expectations preceded each subject's first inspection.
The baseline is the recorded pre-tail working tree based on `ebbc003`. No Java
recompilation, device run or behavioral executor ran.

| Original method, owner Lfixture/TailCompletion; | Judgment |
|---|---|
| `decorated(III)I` | Equivalent on recorded paths; first-stage case breaks retain their original destination without an unreachable second goto. |
| `select(III)I` | Equivalent unchanged protected dispatch, cleanup and later effects. |
| `caught(I)I` | Equivalent unchanged recovery, selected value and post-catch mark. |
| `breakScope(I)I` | Equivalent: a break completes its own switch before mark62/return. |
| `matches(II)Z`, `read(II)I`, `mark(I)V` | Same event-before-result/throw rules, arguments and wrapping trace arithmetic. |

### Fresh positive and protected guards

The first protected select input already had no redundant continuation, so it is
retained as a boundary guard, not reported as positive generalization. The later
decorated subject was added with its own expectations before inspection and does
exercise the same mechanism as the seed: its shared-default first switch is an
explicit graph, with successful discriminator stores followed by case breaks.
The original case0/1 sequences had another planned goto to the default after
those breaks. The candidate omits only that unreachable display action.

For keys0/1/2, matches still runs once with candidate0/1/2 before the comparison.
A match selects discriminator0/1/2 and second-stage value10/20/30. The read then
marks110/120/130, cleanup marks90 and normal completion marks91 before return.
A failed match keeps the original default -1; unknown key skips matches entirely.
Both still read -1, mark99, clean90 and mark91. A selected read failure marks its
read event, cleans90 and rethrows the same captured ISE without mark91. No string,
hash or library behavior is inferred by this implementation.

select's calls/branches are unchanged: a matching case stores its read result and
jumps past the shared default assignment into normal mark30/mark40. A mismatch
or unknown key selects -1 and skips read, including in mode1. A selected read
failure reaches the original catch-all, marks30 and rethrows without mark40.
The enclosing protected completion was not pruned from a descendant throw.

caught still marks50, either selects7 normally or marks51/selects9 after NFE,
then marks52 and returns. breakScope still chooses mark60/61, breaks only that
switch, then marks62 and returns7. The switch's boundary is not treated as an
unreachable parent completion just because its last case ends in break.

### Consumer contract

P7's old continuation omission recognized only a final goto to the same label.
The owning display pass now consumes an already printed unconditional end and
propagates that fact only through trailing Sequence children without an intervening
continuation. It stops before a scoped exit's own target and at opaque containers;
conditional or origin-only terminals are not unconditional printed exits. This
uses delivered layout products, not a second CFG, symbolic replay or output gate.
Original transfers, copies, terminals and source control continuations remain.

The first positive test incorrectly looked for the planned action on a leaf
block. Delivered products place it on the enclosing case Sequence; the test was
corrected to inspect that supplied tail and source continuation. The production
rule was not relaxed to make the test pass.

## Prior effects and bounded decision

Only this fixture and branch-switch-string text change relative to the pre-tail
tree. The seed keeps both hash/equality calls and their order, collision/default
routes, null-before-effect behavior and add-input/mul-literal field identities.
Its unreadable second goto after break disappears, but the original dispatch is
not reconstructed as a string switch. Other corpus text reuses prior reviews.

Within these reviewed paths the repair is effective: the fresh second dispatch
and later throwing read/cleanup retain their effects, while own-switch completion,
protected completion and a necessary release continuation remain. This is bounded
manual evidence, not universal semantic verification or commit authorization.

## Supporting checks and limitations

Four package checks use ordinary Engine delivery: source continuation versus
omitted display placement, one original transfer anchor, effects after an own-
switch break, protected/catch work and a retained release goto when the original
release transfer prints no jump. They do not execute the scenario matrix.

Final default and all-feature nextest each passed 47 package checks. Package
Clippy with warnings denied, release and dbg_shell builds, format/diff and
desensitization checks passed.
Corpus inspection covers 82 fixtures,133 classes,625 methods without stopped
members or render failures/issues. Complete delivered P3/P4/P5/P6 and exact P2
responses agree; only operational elapsed-time fields are excluded from response
comparison. Behavioral execution remains unrun. Faults/concurrency outside the
README model and unreviewed arbitrary inputs remain outside this judgment.
