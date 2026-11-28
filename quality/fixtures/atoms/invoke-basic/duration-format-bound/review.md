# Semantic review

The selected source method passes `Math.max(duration, 0)` to the formatter and
returns the formatter's result. The pinned DEX retains the long overload of
`Math.max` before the one-argument elapsed-time call. The rendered method
from each backend preserves that argument dependency and forwards the result
without another call. [Pinned source method](https://github.com/TeamNewPipe/NewPipe/blob/84d7db5c1348a1a2f73c80970b977d8e1ef6a83d/app/src/main/java/org/schabi/newpipe/util/Localization.java#L270-L272)

- An ordinary negative input and the smallest signed long both become zero. The
  latter exercises the bound without a negation that would wrap at the minimum.
- A zero input reaches the same single formatter call with zero.
- Positive inputs pass through at full width. The maximum signed long tests that
  the value reaches the formatter without narrowing.

The authored control invokes the formatter with the raw duration before
performing the correct bounded call. Its final token agrees on every input. A
trace comparison exposes the extra effect in all five scenarios, including
inputs where both calls receive identical arguments.

The executed evidence covers the modeled formatter's arguments, occurrence, and
result forwarding. Android's locale-dependent formatting and formatter
failure behavior remain outside this review.
