# Semantic review

The original metadata-only method has no explicit monitor operation
and no execution-synchronized flag. Its `Thread.holdsLock` use observes
the unlocked receiver. In the frozen 0.4.14 reconstruction the method
acquires the monitor before querying it, which changes the result.
The physical method still has matching normal and abrupt releases, but
the reconstructed method wraps its entire explicit block in another
monitor acquisition.
[DEX access flags](https://source.android.com/docs/core/runtime/dex-format#access-flags)

The named nested method has two original enter sites for the supplied
object. On a normal or throwing invocation the original path releases
it twice. The frozen reconstruction flattens the nested regions to one.
Both models report a held monitor during the hook and no residual
ownership after the return or throw. The exact recompiled classfile
provides the additional evidence that distinguishes the acquisition
counts.

The 0.4.15 publication keeps each explicit acquisition and release.
The metadata-only subject again queries an unlocked receiver. The physical
subject holds one monitor and releases it on return and on the original
throw. The named nested subject holds two acquisitions and balances them
on either path. The JVM companion retains its implicit method locks and
the separately acquired physical monitor. It checks normal return, the
original throwable, and lock release in actual JVM executions.
[JVMS synchronization](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-2.html#jvms-2.11.10)

The this-lock nesting control retains both regions in the same executable.
The native synchronization control retains both JVM flags with no
explicit JVM monitor instructions or executed native body. The original
proof and Java execution use an initially unlocked monitor, synchronous
modeled hooks and a single supplied fault.
