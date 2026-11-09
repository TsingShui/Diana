# Captured-effect challenge semantic review

Baseline: f83f2fa. Candidate: the accompanying uncommitted source changes.
The same prepared DEX was inspected on both revisions through ordinary Engine.

## Reviewed behavior

combined now retains one shared adjusted local, then nests pair inside the final
tick. Reading that local twice does not reevaluate its arithmetic or the first
tick. The output computes tick(x), then (result+y)*3, then pair(a,a), then tick.
For (3,2), these arguments are 3, (36,36), 396 and the result is 403.
For (-8,2), they are -8, (3,3), 33 and the result is 40.
Input (13,2) increments visits and throws before any pair call.
The independently derived wrapping case (650752614,0) reaches pair arguments
1952257863, producing 13; the final tick then throws after the first tick and pair.
The test-only linear plan executor checks these results and ordered calls.

protectedValue keeps the first tick outside the protected body and its saved
result outside the catch scope. Input 1 reaches tick(1),tick(8), returning 15.
Input 6 reaches tick(6),tick(13), catches the latter exception and returns the
earlier saved 13. Input 13 fails in the first tick and cannot enter the catch.
A draft declaration placed inside try did not cover the outer return; it was
rejected and fixed by following composed Move reads to their final locations.
The final product retains an outer declaration. These protected paths are LLM
source/plan review, not executed catch behavior.

twoArrays still constructs two distinct objects. The first literal [1,2,3]
precedes sink and remains the object published to observed. The second [4,5,6]
is constructed afterwards and returned. Neither payload substitutes for the
other; both original allocation/fill anchors remain. Object identity and
allocation failures were not executed on a device.

## Evidence limits

Numeric helper behavior is modeled from the supplied bodies; constructor,
resolution and allocation failures of the runtime are not modeled by that
executor. Structural/origin checks and bounded semantic reading support this
challenge, not a general final-plan or renderer correctness claim.
