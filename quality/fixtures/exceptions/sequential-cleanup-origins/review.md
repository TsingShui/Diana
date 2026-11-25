# Cleanup operand review

Before the operand-matching repair, sharing SSA facts across regions left
class text byte-identical for this fixture. Comparison used the ordinary CLI
`get-class` command. The following path review compares that output with the
disassembled DEX and the independently specified helper model. Native helpers
were modeled for the reading and were not executed.

## Matching cleanup copies

In `closeBoth(II)I` the first try reads the first parameter and its finally
closes that same parameter. Failure of the read therefore runs exactly one
close before propagating E. A throwing first close propagates C and prevents
entry to the second try, including when that close replaces E.

The second try reads the second parameter. On success it adds that result to
the saved first result. Java evaluates the sum before entering the second
finally while DEX adds after its normal close. Adding these two captured ints
has no observable effect and cannot throw. The call to close therefore
precedes delivery of the same sum in both forms. A failing second read skips
the sum, closes the second parameter and propagates E. A failing second close
propagates C after either read outcome. The two finally operands in the output
match the two original helper arguments on every modeled path.

The Engine regression inspects both try bodies and their respective finally
operands through `Engine::analyze_method`. It identifies the two operands
from the method signature. This rendering check passed in the candidate nextest
run (one test, zero skipped). The scenario exits and
effect ordering above were reviewed by reading the text.

## Mismatching cleanup copies

In `readBoth(II)I` the first try, first close and faults before entry to the
second try agree with the original method. On failure of the second read the
DEX handler closes the first parameter. The second finally in the output
does the same, preserving both propagation of E and replacement by C.

The original normal successor of the second read closes the second parameter.
Both pre-fix builds instead render a second finally that always closes the first
parameter. With the fixture inputs a successful second read produces
`close(10)` where the DEX performs `close(20)`. This changes an observable
helper argument even on the fault-free path. A fault selected specifically for
the normal close of handle 20 is unreachable in the rendered output, which
returns 10 instead of throwing C. This pre-fix mismatch reproduces the incorrect merge.

The operand-proof repair in the 0.3.0 candidate retains the first finally
and emits the second read inside a try block. Its catch closes the first
parameter and rethrows the captured exception. The normal path closes the
second parameter before adding the two read results. A normal close failure
is outside the catch, so it escapes directly. A failing exceptional close
replaces the read exception, and every failure in the first region prevents
the second read. The positive closeBoth case retains both finally regions.
The [ordered argument challenge](../ordered-cleanup-arguments/review.md) tests
this boundary when both cleanup copies start with the same effect.

## Parent fixture

Both builds also produced identical `get-method` output for
`Lfixture/ReassignedResourceFinally;->rotate(I)I` on its existing DEX.
The seed read and initial open precede the try, so an acquisition fault
cannot trigger a close. Once acquired, a failed read closes the active handle
through the handler. In mode 1 the code clears ownership before closing the
first handle and sets it again only after opening the replacement. A failed
old close or failed second open therefore avoids a duplicate close. A failed
replacement read closes the replacement. On a normal return the active handle
is closed before delivery of the saved first read or the sum. A failing close
propagates its own throwable, including when replacing a read failure. The
parent fixture's modeled paths retain their original effects.

A JVM execution check of the exported class supplied the modeled native
helpers and ran the seven fault selections for each method. Before the
operand repair, the normal second close in `readBoth` had the wrong argument
and its selected failure never occurred. All fourteen cases passed in the
repaired CLI, including the original positive `closeBoth` paths.
