# Handler cleanup boundary review

I compared the DEX disassembly with the original and corrected get-method
output. The input marks acquire and probe outside the read range. Its IO
handler has exactly one protected operation, log. The successful continuation
of log performs audit and the conditional close with no handler. Control
reaches the catch-all cleanup if read throws a non-IO exception or log throws.

The pre-fix output ran a finally close as soon as log completed and then
continued to audit and a second close. This changed the order even when all
helpers succeeded. It closed the handle before a failing audit. A close
fault could skip the audit entirely.

In the corrected output acquisition fault I and probe fault P both leave
before the read. On read success the result is captured before one close,
and a close failure C prevents delivery of that result. For read failure I,
the original failure reaches log. A logging failure L runs one close before
rethrowing L. C from that close replaces L. Successful logging runs audit
before exactly one close and returns -2. Audit failure A escapes before the
close, and C on the post-audit close escapes without retry. For non-IO read
failure R, the typed handler does not run. One close either returns before R
is rethrown or replaces R with C.

## Executed checks

The fixture's JVM helper substitution ran on OpenJDK 21.0.12.1. It retained
the recovered method body and supplied exception declarations in the Java
signatures. The reference passed all eleven cases. The pre-fix candidate
failed the successful IO recovery, audit fault and post-audit close fault
cases. The corrected candidate passed all eleven traces, exits and throwable
identity checks. The behavior of external helpers is limited to the explicit
fixture model.

The two ordinary Engine tests in handler_cleanup_boundary_test failed on the
pre-fix source. Both passed on the corrected source, together with all three
sequential cleanup operand tests. These checks inspect publication through
Engine independently of the JVM helper execution.
