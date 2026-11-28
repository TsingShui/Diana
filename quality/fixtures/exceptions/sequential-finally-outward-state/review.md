# Semantic review

The original DEX's stage 1 effect occurs after the timestamp has been
set to 3. Stages 2 and 3 occur after the conditional assigns 1 or 2.
All stage failures execute the exceptional cleanup, and all normal
stage paths execute the normal copy. Both cleanup copies pass the
original receiver and token. The outer catch observes the timestamp
and exact stage or cleanup throwable. A reporter fault escapes.

The 0.4.2 publication drops the assignment of 3. It uses 0 if stage
1 fails, whether the outer catch receives the pending stage fault or
a replacement cleanup fault. It preserves both stage 2 traces,
including the route-dependent state and exact outward throwable. It
omits cleanup on all normal completions and stage 3 exceptions, so
those paths either bypass the outer catch or enter it before cleanup.

The tested release and debug candidates both recover the timestamp
assignment and branch merge before the relevant effects. The nested
finally covers all three calls and uses the original receiver and
token. The outer catch runs after cleanup and receives the original
or replacement fault without changing the observed timestamp.
The exact recovered method text agrees with the original DEX and the
independently authored recording model on all ten scenarios.
