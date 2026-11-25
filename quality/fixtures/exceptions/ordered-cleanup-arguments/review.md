# Ordered cleanup review

I read the assembled DEX disassembly, the pre-fix Java and the candidate
Java from the ordinary get-method command. Both DEX catch-all ranges cover
only the read and its result move. A failure in any cleanup helper therefore
escapes directly. The candidate keeps the shared first finally and gives the
second read an explicit catch with its own cleanup.

On success the first read returns 3, the first close receives 10 and 20,
and the second read returns 7. The normal mark consumes the saved 3, then
the final close receives 20 and 10 and the caller receives 10. Throwing from
the first close prevents the second read. Throwing from the normal mark skips
the reversed close. Throwing from that close prevents the return.

A failing first read still closes 10 and 20. A failing second read executes
the handler mark with the saved 3, closes 10 and 20, and rethrows the same
read exception. If a handler close fails, that close exception escapes. If
the handler mark fails, its exception escapes without a close. The candidate
keeps these effects and exits on all nine modeled scenarios.

The pre-fix candidate folded the two second cleanup copies into a single
finally. It always closed 10 and 20. This changed the observable trace even
on normal completion. It also returned 10 for the fault configured specifically
on the final close of 20 and 10.

## Executed checks

The fixture harness substituted its three modeled native helpers in the
ordinary Java output and ran the recovered method on OpenJDK 21.0.12.1. It
compared every helper argument in order, the returned sum and the identity of
the escaping throwable against the fixture scenarios. The reference Java
passed nine cases. The pre-fix output failed the two normal-close scenarios
described above. The fixed candidate passed all nine.

The three ordinary Engine regression tests passed through cargo nextest. They
check the positive two-finally case, the differing single operand and the
ordered two-argument challenge. Runtime checks use Java with modeled helpers.
The DEX was inspected through disassembly without executing it on ART.
