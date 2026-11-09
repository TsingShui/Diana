# Shared cleanup context review

## Natural catch display

Only `resource` changes. Its outer `IOException` catch already prints both routes,
so the chain line, the inline markers on acquisition, both normal closes,
`addSuppressed` and `throw`, and the `H18` label are omitted. Normal close outlets,
the nested all-catch, distinct primary/secondary reads and suppression order are
text-identical. `split` keeps its explicit chains because its crossing contexts
remain explicit. Original P2–P6 source products are exact, and text/JSON publish
together. This is a renewed LLM reading of the exact before/after text plus product
equality, not executed behavior.

## Verdict

The challenge was authored after the shared-frontier refinement and before its
first inspection. Compared identical bytes on fixed baseline 73fc8793a91d766d021d117aa83ddfd644f7ac82
and the repaired ordinary Engine chain. Read both sources, every pre-inspection
scenario and complete outputs. Split recovers the enclosing all-catch and
acquisition catches without merging its distinct capture entries. Resource
recovers its body and nested suppression catch while retaining outer IO recovery.
No contradiction was identified under the stated models; unknown native/library
behavior is not promoted to an established fact. No behavior was executed.

## Split contexts

Normal acquisition commits H, then ready precedes read. False ready and negative
read take their original empty-result close and return-1. Nonnegative success
captures the read before its different normal close and return. Failure at either
normal close replaces that result and cannot run cleanup again.

Before acquisition commits, the pre IO/interruption adapters select null.
Later adapters retain H. The shared interrupted terminal therefore has both
pre/post arrivals but closes only the original selected receiver when rethrown
through the pre all-catch. There is no log on interruption. IO takes the shared
log; normal logging closes H when present then returns-1. If logging fails after
acquisition failure, the post all-capture still receives null and skips close.
After read failure it receives H and closes before propagating the logging
failure. The original exception, not a manufactured value, supplies each capture.

The two all-captures still have their own receives and assignments to the shared
cleanup receiver. Their shared pure guard at PC47 is outside both the protected
body and selected handler. They jump there rather than reentering the try body
or falling through the success close. Null acquisition fails at ready and skips
close after retaining that null receiver. A cleanup failure replaces the pending
throwable; there is no addSuppressed in split.

The post-acquisition typed entries remain explicit: their shared logging/rethrow
contexts do not establish a separate natural lexical scope. Keeping those actual
entries is preferable to duplicating handlers or inventing a discriminator.

## Resource outlets

Acquire precedes resource-body protection. Read success commits its value before
the predicate and note. Positive success calls note, then closes and returns the
captured value. Nonpositive success skips note, closes and returns0. These normal
closes are outside the resource-body catch, so an IOException there selects the
outer recovery directly, with one log/return-7 and no second close/suppression.

Read/note failure selects the original primary capture. A present H closes once;
a secondary failure is caught separately and passed to the original primary's
addSuppressed. Success then rethrows the same primary. An IO primary reaches the
outer IO recovery even after a distinct suppression; an unmatched E propagates.
Self-suppression rejects with S before the later primary rethrow. A logging failure
propagates without any cleanup retry. The outer recovery is not private body
content simply because discovery reached it through normal-close exception edges.

## Evidence and rejected drafts

Two ordinary-Engine checks require two split scopes, six original captures,
separate all-entry receives, the outside shared guard, three resource scopes,
normal-close boundaries, outer-recovery separation and distinct suppression
reads. The nine-check iteration set is baseline-sensitive: two refusals pass,
seven recoveries fail on the baseline; all nine pass on the candidate.

A broad terminal-only nomination draft was rejected: it obscured existing
finally/resource nesting and mistook a recovery loop's reentry for a cleanup
outlet. The final producer uses established same-local capture contexts, keeps
primary/secondary locals distinct and consumes existing loop reentry facts.
The earlier positive/negative source expectations were not changed to accept
that draft. Full isolated default/all-feature runs pass 155 checks, and the
111-fixture source-fact comparison is exact with no retrieval/render failures.
These are product and manual-review results, not a fixture/APK execution PASS.
