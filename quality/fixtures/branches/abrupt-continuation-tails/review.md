# Fixture: abrupt-continuation-tails

## Current verdict — constructor argument and throw recovery

Compared the same prepared DEX on `e95b6a4` and the containing repair. Read the
original Java and prepared DEX behavior and complete before/after output for caught(I)I.
Only construction spelling changes; other member output is exact.

### Semantics

mark50 precedes the flag test. Flag0 allocates/initializes and throws NFE through the same ordered c0 chain, then mark51 selects9. Other flags select7 without construction. Both paths mark52 once. Constructor failure keeps its actual handler/type dispatch rather than being replaced with an initialized NFE. Explicit sharing and later work remain.

Original P1/P3/P4 publications agree. P5 retains allocation, arguments and normal
publication with the same ordered protection; P7 composes an adjacent throw only
with equal handler chain/spelling and no snapshots. Original sites and effect
order stay delivered, including explicit chains when lexical catch delivery is
unavailable. The [fresh argument/effect challenge](../../atoms/constructor-argument-effects/review.md)
covers trees, fault prefixes and order/sharing/handler/save refusals.

LLM path review found no contradiction under the fixture's recorded helper and
external-constructor assumptions. Unchanged paths reuse the earlier independent
review below. This is renewed semantic reading plus current product regressions,
not a full-method VM/device or Java execution result. Historical execution claims
below are not rerun certifications.

## Prior verdict — one external discriminator tail

Compared the same prepared DEX on dd70d20 and this candidate. decorated now has
private first-dispatch cases and one external -1 assignment. A successful match
sets the original discriminator and jumps past that assignment to the second
switch; a failed match or unknown key reaches -1 once. Thus matches still marks
80/81/82 only on the selected case, then read records110/120/130 or99, cleanup90
runs once, and mark91 runs only after normal read completion. The goto enters
the original second-dispatch label; no selector, read or tail is duplicated.

Direct LLM reading covers the original Java/Smali scenarios and complete changed
method. Its old raw shared-default shape is superseded, not its expected events.
The test now checks one original anchor and no second continuation for every
printed unconditional exit, even when precise P6 ownership already omitted the
old source Goto. breakScope/caught/select remain unchanged and reuse prior review.
Original P2/P3/P4/P5 facts and copy phases agree. The
[fresh shared-tail challenge](../switch-shared-tails/review.md) adds downstream
bypasses, real selected assignments and nested protection. No behavior execution
or Java/device run is claimed.

## Prior verdict — terminal guard follow-up

Read the changed terminal alternatives and remaining body. The selected
return/throw keeps its original evaluation, while other copies, calls, accesses
and results remain on the opposite path in their original order. Both-edge guard
orientation preserves the predicate's exact meaning; no data, check or effect is
dropped.

Compared the same prepared DEX on 29f1b82 and this commit. P7 requires a complete
linear arm ending in a printed return/throw; opaque scopes are not exit evidence.
The [fresh guard challenge](../../branches/terminal-guard-arms/review.md) covers
actual other-edge Phi copies, parent scope, recovering catches and cleanup.
Original P2 through complete P6 products agree; this is bounded LLM path reading,
not behavioral execution. Historical execution claims below were not rerun.


## Prior verdict — whole-operation follow-up

Read both changed dispatches against Java/Smali. select keeps matches/read order
and selects -1 only on the original mismatch/default path; successful read
bypasses that assignment via the supplied Try Break, then cleans30 and marks40.
decorated still performs its first dispatch, shared read, cleanup90 and
normal-only mark91. Read failure cleans and rethrows the same object without the
later mark. Matched finally retains the original implicit rethrow anchor; the
shared non-case graph is not fabricated into a switch.

The [pure-tail challenge](../../exceptions/pure-tail-finally/review.md) and
[carrier challenge](../../atoms/refined-carriers/review.md) record the owning
producer/consumer repairs and blocked drafts. This is bounded current source
reading; historical execution claims below were not rerun and do not certify
this output. No behavioral executor, device run or Java recompilation ran.


## Prior verdict — protected-entry follow-up

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
