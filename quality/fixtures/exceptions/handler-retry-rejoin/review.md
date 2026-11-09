# Captureless retry review

## Natural catch display

Only the outer `IllegalArgumentException` catch around `fast(p0)` changes: its
chain line, inline marker and `H2` are omitted because the printed try/catch plus
method-level propagation already state both routes. Fast success still returns
before fallback; the fallback while, `reset`, `attempt`, shared increment and the
still-explicit inner `IllegalStateException` chain are text-identical. Original
P2–P6 source products, sites and receives are exact. Text and JSON change together; the owned sites, chains, completions,
snapshots and receives remain. This is a renewed LLM reading plus product
equality, not executed behavior.

## Prior verdict — P6 captureless recovery

The same prepared bytes now recover one while against baseline 73fc879.
Direct reading of the source/model scenarios and complete output retains fast
success, fallback-only row initialization, reset before attempt and one shared
increment. Normal failure and the ignored exception reach that increment once;
only a normally committed matching result reaches the outside success return.
Exhaustion returns-1 and unmatched/reset failures still propagate. Original
captureless entries remain captureless, with no result fabricated on failure.
P2–P5 and original P6 coordinates agree. The new recovery check passes alongside
the preservation check; the [fresh challenge](../captureless-retry-nesting/review.md)
adds nesting, effects, latch entry and genuine foreign refusal. This is bounded
LLM reading plus product regression evidence, not executed behavior.

## Scalar-use baseline observation

The prepared reduction reproduces R1's explicit `graph<Irreducible>` loop on
baseline df10b744985e3407f902442b8252fab100700aec. Both ignored handlers lack
move-exception; the inner handler starts at the same increment instruction
entered by normal failed attempts. A preliminary captureful draft recovered a
while and was rejected as a reproduction of R1.

The scalar-use rendering repair leaves this text and every P2–P8 product
unchanged. The separate P6 repair above supplies natural loop ownership.
The check preserves original captureless/shared-entry facts without requiring
this inferior shape forever.

### Semantics

| Method | Judgment and reasoning |
|---|---|
| `run(III)I` | ~ Fast success skips fallback entirely. The graph initializes row0 only after the selected fast exception. Each entered row tests the original limit, resets before attempt, then either returns the matching result or advances once. The ignored inner exception enters the same increment without reading the failed attempt's absent result. Exhaustion returns-1. |
| `fast(I)I` | ? Native trace/return/IllegalArgumentException behavior is modeled in README, not executed or supplied as Engine facts. |
| `reset(I)V` | ? Native trace and selected outside-try fault behavior are modeled. Its call is outside the attempt chain, so its failure cannot turn into another retry. |
| `attempt(II)I` | ? Native result/selected IllegalStateException behavior is modeled. The original chain handles that type only; nonmatching failures propagate without another increment. |

For run(2,12,3), row0 fails the value test, row1 fails the attempt, and row2
returns12. H8 advances the first two paths once each. It does not rerun reset,
re-enter the failed invocation, fabricate a result or capture an unused throwable.
With limit0, fallback returns-1 without reset/attempt. A nonmatching fast Error
propagates before fallback, and a reset failure propagates before the attempt.

This is LLM semantic reading under explicit models, supported by original handler
and normal-link checks. This baseline observation did not establish natural
recovery. No fixture/native execution is claimed.

## Rounds

- Reproduced captureless handler/shared-update refusal; retained as an unchanged
  control/availability guard while repairing scalar-use text.
