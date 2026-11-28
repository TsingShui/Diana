# Typed catch cleanup return review

The monitored method starts with no result. An empty traversal performs
cursor close and owner close and returns null. A nonempty traversal captures
the original input, runs the body action, and then performs cursor close.
The normal return retains the captured object. Diana 0.4.29 and the candidate
agree in these scenarios.

A failure in the body before cursor close enters the inner Throwable handler.
It closes the cursor and rethrows the original failure. A distinct close
failure is suppressed on the original failure. Self-suppression raises a
new exception before the rethrow, which also goes through the outer typed
catch. A failure from normal cursor close enters that outer catch directly.
The outer catch selects null for all those paths. Owner close still runs,
and an owner-close fault overrides the pending result with its original
exception object. The monitor is released after owner close in every path.

Diana 0.4.29 publishes the typed handler with an empty body in the reordered
layout. The subsequent normal return can then read the original nonempty
capture. The revised publication keeps `return null` inside that handler.
The owner close appears once in its finally, with the monitor enclosing the
entire try.

The handler's physical close block may be contracted to its established
completion. The revised transfer retains the original edge when it reaches
the recorded handler continuation and the proved cleanup completion.
The following monitor release can lead to a terminal in the lexical parent
when it is already represented by the synchronized scope and there is no
intervening evaluation or unresolved merge. Materialized terminal values and
cleanup still pass through the ordinary region exit handling.

This review covers the stated layouts, exception routes, and results. Its
observations are separate from the executed modeled behavior checks.
