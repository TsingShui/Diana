# Natural typed catch display review

## Natural catch display

`multi` now prints without its three-row chain or markers. Both printed catches
keep their original order and bodies, `return -2` and the unreachable-after-throw
`return v1` are unchanged, and the separate `foreign` refusal keeps an explicit
chain. The [nested challenge](../nested-catch-display/review.md) covers the
enclosing-same-type, catch-body and union shapes. Text and JSON publish together; owned sites, chains, completions,
snapshots and receives remain. Renewed LLM reading and product equality, not
executed behavior.

## Verdict and scope

The fixture was authored after the display draft and before its first inspection.
Compared identical prepared bytes on 1d5f508870b2de2a0b3e55dc2dfd41b55b2a52cc and
the P7 repair. On the baseline, `run` and `rethrow` print a redundant chain line,
two inline `[exn c0]` markers and an `H` handler label even though the visible
`try/catch` already states both original routes. The candidate prints only the
`try/catch`. `foreign` and `multi` are text-identical. No contradiction was
identified under the pre-inspection models; no native, VM, device or Java-output
execution occurred.

## run

`v7`/`v3` initialization, the `while` head, `step(v7,p1)`, the sum commit,
`recover(v4,v7,p1)`, its sum commit and the `+1` update keep their order and
operands. A matching A thrown by `step` still enters the catch without committing
a step result; the catch's recovery value is added once. Unmatched E and a failing
recovery still propagate without an update or later iteration. Nonpositive limits
still perform no call. The chain-and-marker removal changes no printed action, and
the original ordered sites, chains and receives remain in the product with one
anchor each.

## rethrow

The unused `step` effect remains a statement before `throw p1`; the catch still
returns `recover(v3,0,p0)`. Both routes of the platform throw therefore stay
visible: a matching A reaches the catch, while E, K and the modeled null N
propagate out of the method. The candidate does not delete the prior call because
its result is unused, and it does not add a branch.

## foreign refusal

Two separate protected intervals share one handler, with `outside(p0)` between
them. Because that effect is unprotected, an A raised there must escape even
though the handler's type matches. The candidate keeps the explicit chain, inline
markers, `on_exception_entry` and snapshot text. It does not merge the intervals,
duplicate the handler or absorb `outside`. Mode0 still returns through the first
catch, mode1 reaches the second, and a normally completing `outside` contributes
no catch.

## multi refusal

The broader two-handler chain retains its original three ordered clauses and the
printed both-catch layout. This first narrow rule does not claim that the
remaining typed specialization is fully represented there, so the markers and
chain line stay. `B` still returns -2 without recovery; `E` still propagates.

## Corpus and APK observations

Eleven previously reviewed fixtures also lose only this redundant notation; each
changed path is recorded in its own review. Complete P2–P5 products and original
P6 node/edge/link/clause/completion/copy/lock coordinates are unchanged across
116 fixtures. The selected APK methods `DateUtils.parse` and
`ChapterUtils.loadChapters` lose the same repeated markers, and `parse` separately
composes its adjacent `SimpleDateFormat` check into the following read; the other
four selected class texts are unchanged. Because the inline annotation was the only
remaining obstacle, that cast is now printed on the value it checks rather than as
a separate statement. The check still executes once at its original position with
its own origin anchor and the same handler. Whole-APK coverage and failure counters
are unchanged. These are product and manual-review results, not execution.
