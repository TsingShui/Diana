# Semantic review

The original raw read at code unit 0 of each external consumer getter
targets one of the interface fields. The independent replay checks the
unencoded public static final declarations and runs their initializing
sequence at the first access. The same-thread callback sees FIRST after
the first write and SECOND before the second write. Both fresh Java
controls publish the same full ordered trace.

The 0.4.13 FIRST-first reconstruction publishes
`before,selected:first,during:first:null,first:first,second:second`.
The expected trace starts
`before,during:first:null,selected:first`. The unchanged return value
would hide this effect-order difference. The SECOND-first control
requires the effect to obtain its value and preserves the original
trace in the same version.

The 0.4.14 candidate retains an active read of FIRST and commits the
first public field before the callback. Both complete reconstructed
classes recompile with the unchanged observer and hooks and pass both
traces. These runs exercise the two modeled same-thread reentry
scenarios.
