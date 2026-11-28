# Semantic review

Reviewed `e0:l0:c0:m1` in Diana 0.4.3 against the original DEX, the
independently authored Java control and all six scenario traces.

The DEX normal path crosses all three ordered calls, then invokes
`cleanup` with the input receiver and token before returning. The
publication returns immediately after the third call. It therefore
skips the cleanup and any exception that the cleanup would raise.

On an exception from stage 1 or 2, the original entry forwards the
same throwable to the common suffix. The publication still passes the
original receiver and token to cleanup, but its catches discard the
pending throwable and complete normally when cleanup succeeds. On
a stage 3 exception, the publication propagates the original throwable
without entering cleanup. The original always executes the cleanup
first. If stage 2 and cleanup both throw, both versions propagate the
cleanup fault because it replaces the pending stage fault.

The modeled traces establish those differences for the authored helpers
and scenario inputs. The publication reports no recovery diagnostics.

The cleanup-family candidate contains the three stage calls, in order,
inside one protected body. The generated return is inside that body,
so successful completion runs `finally` before returning. No clause
swallows an exception from an individual stage. The shared cleanup uses
the same receiver and token, and a failure from cleanup replaces the
pending exception. These paths match the independent model and were
executed by DecBench.
