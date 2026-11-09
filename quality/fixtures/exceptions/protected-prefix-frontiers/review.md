# Fixture: protected-prefix-frontiers

## Current verdict — scoped catch capture

Compared the same prepared input on `c95c6dd` and this repair. Reviewed header
bindings: `NumberFormatException | NullPointerException v5`. The original captured object is bound once at exceptional
entry, before the unchanged receiving/body actions. Its exact-state uses remain
inside that handler and have no capture-phase, Phi or copy dependency requiring
an outer assignment. Normal execution does not bind or run the handler.

After accounting for that binding and its removed private declaration/assignment,
the complete remaining output is exact. Original P1/P3/P4/P5 products agree.
Thus the earlier independently reasoned normal, caught, cleanup, rethrow and
secondary-failure paths below are reused: their calls, arguments, values,
priority, transfers and effect order did not change. The renewed LLM judgment
finds no contradictory capture lifetime or exception identity. Structural
comparison supports this reading rather than supplying behavioral proof.

The [fresh monitor/binding challenge](../../sync/monitor-surface-boundaries/review.md)
covers capture identity, receiving/value flow, nested failures and escaping
storage. Current fixture/model regressions are separate executed evidence;
this record does not claim VM/device execution for this input.

## Prior verdict — terminal-follow conjunction, 2026-11-09

`cleanup` now prints `p1 == 5 || p1 == 6` before its `IllegalArgumentException`;
`read` prints `p1 == 4 || p1 == 6` before its `AssertionError`. The successful
`p0 + 1` result and both throw/return paths are unchanged; the rest of the fixture
text is unchanged in the candidate comparison.

## Current verdict — terminal guard follow-up

Only read helper guards change, preserving each mark/result/fault and mode. Both
natural prefix ownership and genuine multi-entry explicit protection remain
unchanged.

Compared the same prepared DEX on 29f1b82 and this commit. P7 requires a complete
linear arm ending in a printed return/throw; opaque scopes are not exit evidence.
The [fresh guard challenge](../../branches/terminal-guard-arms/review.md) covers
actual other-edge Phi copies, parent scope, recovering catches and cleanup.
Original P2 through complete P6 products agree; this is bounded LLM path reading,
not behavioral execution. Historical execution claims below were not rerun.


## Prior verdict — whole-operation follow-up

Read both final methods. branch retains both guarded reads, their common pure
prefix and outer-only lead calls; natural finally runs the original cleanup
once. split retains genuine multiple-entry protection and both reads/one shared
handler in an explicit graph. Normal cleanup's changed graph-container placement
adds no catch semantics; its original link/order and later mark50 remain, while
failures keep their original delivery.

The [pure-tail challenge](../../exceptions/pure-tail-finally/review.md) and
[carrier challenge](../../atoms/refined-carriers/review.md) record the owning
producer/consumer repairs and blocked drafts. This is bounded current source
reading; historical execution claims below were not rerun and do not certify
this output. No behavioral executor, device run or Java recompilation ran.


## Prior LLM semantic review

Compared Java/Smali, concrete helpers and final ordinary Engine output on the same
DEX. Independent expectations precede first candidate inspection. Baseline is the
recorded pre-prefix working tree based on `ebbc003`. No Java recompilation,
device run or behavioral executor ran.

| Original method, owner Lfixture/PrefixFrontiers; | Judgment |
|---|---|
| `branch(III)I` | Equivalent on recorded paths; the two-read typed core owns its common flag test, not the foreign throwing prefixes. |
| `split(II)I` | Equivalent explicit recovery; two real core entries and one original handler remain, without a fabricated private typed scope. |
| `mark(I)V`, `read(II)I`, `cleanup(II)V` | Same arguments, event-before-result/throw order and wrapping arithmetic. |

### Positive branched core

The initial seed had one guarded read and one optional goto prefix. This challenge
has two guarded reads, so merely retaining an unexpanded base would not work.
The candidate grows the shared nonthrowing flag header and retains that complete
single-entry checkpoint. Lead0/1/other still marks1/2/3 before the inner try. The
later optional goto predecessors remain outside; their throwing lead calls have
only outer cleanup protection and do not acquire the typed catch.

Flag0 reads row0 and selects1; other flags read row1 and select2. Both execute
mark40, cleanup(row2) marking32, then mark50 before return. NFE/NPE from either
read enters the one shared handler, marks20 and selects100, then executes that
same normal suffix. The capture/body is not duplicated for the two reads.

ISE/Error bypass typed recovery and enter the outer catch-all. Cleanup32 succeeds
for those modes, then the same captured object propagates without mark40/50.
Mode5's normal cleanup failure occurs after mark40 and outside the body handler,
so cleanup is not retried and mark50 does not run. Mode6 cleanup replaces the
pending Error with IAE, without suppression. Source scopes preserve that order;
no finally spelling or external hierarchy fact is assumed to force qualification.

### Genuine multi-entry negative

split's prefix60/61 calls are throwing portions with only outer cleanup protection.
The desired typed core has two original read entrances and cannot own those
prefix calls. The constructor therefore delivers no private two-clause TryPlan:
both reads, the one shared capture, mark20 and the precise original site chains
remain in the explicit ProtectionBoundary graph.

Flag0/other still reads row0/1, selecting1/2 normally or100 after NFE/NPE. Normal
mark40 precedes cleanup33, then mark50. ISE/Error bypass typed recovery, clean33
and propagate. A normal cleanup failure or mode6 replacement has no retry or
mark50. No guarded seed was dropped merely to achieve one entry; no handler was
cloned into separate Java catch explanations.

## Owning repair and input guarantees

The old producer expanded every eligible predecessor before asking for one body
entry. In the seed, adding a goto whose other predecessors were outside turned
an established read entry into two entries; no typed scope was published, so
mixed explicit/natural delivery correctly withdrew the outer owner.

The replacement private prefix constructor retains every original guarded seed
and forward body, grows the same nonthrowing predecessor closure once, and tracks
outside normal/executable-exceptional arrivals plus method entry. It records
complete single-entry checkpoints and selects the latest before guard/handler/
ownership admission. Temporary multi-entry growth can still reach a later common
prefix. Dropping a later optional predecessor suffix removes no guarded seed or
successor path. A later refusal does not retry another checkpoint or owner.

A method-private cardinality/XOR aggregate identifies the unique frontier only
when its cardinality is one; it is not a hash-based verifier. Scratch/counter
arithmetic and allocation remain fallible. No new normal dominance, graph replay,
external declarations, product export or method-analysis route was added. The
old prefix path and separate body-entry collection were removed together.

## Prior effects and bounded decision

The seed CaseEntryJoins.route now delivers both its typed scope and outer cleanup
owner with the same values, prefix/flag effects and pending-object order. The
previously explicit decorated read in abrupt-continuation-tails now has a natural
catch-all; pure second-switch selection can be a prefix without adding a throwing
event. First-stage matches, selected values, cleanup90 and normal mark91 are
unchanged. Its first non-case-sharing graph remains explicit.

All three changed fixture texts were directly reread. Other corpus text reuses
prior reviews. Within those paths the repair is effective: the fresh two-read
core requires real prefix completion, and the negative retains both entries and
one shared handler. This is bounded manual semantic evidence, not universal
verification or commit authorization.

## Supporting checks and limitations

Four `protected_prefixes.rs` checks consume ordinary Engine delivery. They cover
the seed entry, both guarded sites under the common flag test, exclusion of
outer-only throwing prefixes, real multi-entry refusal and complete outer scope
coverage at original method entry0. Calls/captures retain one original anchor.
They do not execute the scenario matrix or independently verify arbitrary code.

All-feature nextest passed55 package checks and package Clippy passed with
warnings denied. Corpus inspection covers84 fixtures,135 classes,640 methods with
no stopped member or render failure/issue. Exact P2/P3/P4/P5 and relevant original
P6 nodes/blocks/links/chains/handlers/locks and synchronized targets/routing/phases
agree. Only the three reviewed texts change. Selected body/entry/protection and
ownership products intentionally gain facts; they are not input-byte changes.

Final default and all-feature nextest each passed55 checks. Package Clippy with
warnings denied, release and dbg_shell builds, format/diff and desensitization
checks passed. The fixed-ebbc003 comparison also preserves original facts across
all nine repairs. Behavioral execution remains unrun; faults and concurrency
outside the README assumptions and unreviewed arbitrary inputs remain outside
this judgment.
