# Captureless retry review

## Current verdict

The prepared reduction reproduces R1's explicit `graph<Irreducible>` loop on
baseline df10b744985e3407f902442b8252fab100700aec. Both ignored handlers lack
move-exception; the inner handler starts at the same increment instruction
entered by normal failed attempts. A preliminary captureful draft recovered a
while and was rejected as a reproduction of R1.

The scalar-use rendering repair leaves this text and every P2–P8 product
unchanged. Natural loop recovery is still an independent P6 investigation.
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
and normal-link checks. No fixture/native execution or natural-loop repair PASS
is claimed.

## Rounds

- Reproduced captureless handler/shared-update refusal; retained as an unchanged
  control/availability guard while repairing scalar-use text.
