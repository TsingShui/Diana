# JVM static initializer evaluation order review

The original earlier-operand body calls `capture` before `earlier`.
The latter is evaluated first inside the final consumer expression. The
capture has already completed by then. The original result trace is 123
and the capture fault wins when both producers can fail. Diana 0.4.26
recompiles to trace 213 and chooses the earlier-operand fault instead.
The original selected method publication retains a separate capture
statement. The difference occurs in the full class publication.

The conditional tests its predicate after the capture. On the false branch
it uses the other result, but the capture still runs and can throw.
0.4.26 places the capture in the true arm. Its false result preserves
the returned object while omitting the capture effect or its fault.
Its true branch runs the predicate before the capture, producing trace 21.
The short-circuit initializer makes the same scheduling mistake. It moves
the eager Boolean capture to the right operand, skipping it when the
left operand is false.

The protected initializer assigns the field before entering the typed
exception range. That range contains the later observation of the field
and catches exactly its fault. The normal and fault traces are 1234 and
12345. Both prior producers have the same exception class as the later
fault, but failures there escape with their original identities. In
0.4.26 the initial two effects change order. The typed catch still
receives the later fault on successful publication, while competing
pre-try faults select the wrong instance.

The first-operand control evaluates the capture at the start of the
consumer argument list. The compact field expression in 0.4.26 preserves
the original successful trace and first fault. This is a compatibility
control for retaining compact recovery when the order is proved.

The 0.4.27 candidate keeps the earlier-operand and short-circuit captures
in their static blocks. The conditional interface uses an initializer helper
that captures the value before evaluating the predicate. The protected
initializer publishes its field before the typed range and observes the
same post-publication fault. The recompiled outputs restore the original
normal traces and earliest fault identities. The first-operand control
keeps its compact field expression and both original outcomes.
