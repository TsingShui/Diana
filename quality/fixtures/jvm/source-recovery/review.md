# Review of the compiled source fixtures

OpenJDK 21.0.12.1 compiled the fixture as Java 17 bytecode. `javap` showed
the original argument slots, descriptors and instruction offsets. `Check.java`
executed the generated class and passed its positive, negative and boundary
cases.

`<init>` invokes the superclass constructor before storing its integer
argument. `calculate` reads that field before adding the argument. The
intermediate local carries that sum to `twice`, whose multiplication happens
before the subtraction and the return. The branches in `choose` use the
same field and argument on each path and differ in the arithmetic operation.
Zero uses the subtraction path. `descendingSum` tests the remaining count
before each iteration, adds it to the accumulator, then decrements it. Zero
and negative inputs return the initial accumulator unchanged.

The protected region in `divideOrDefault` ends before the success return.
Division by zero reaches the typed `ArithmeticException` handler and returns
`-7`. On success the integer quotient stays on the stack until the return.

The switch at byte 1 in `classify` targets byte 44 for keys `-7` and `40`,
byte 47 for `6`, byte 56 for `100`, and byte 50 for the default. The ordinary
JVM `get-method` result sends both shared keys to the return of `61`, retains
the separate `-4` and `99` cases, and applies addition only to the default
path. The reference execution covered all four outcomes.

`update` stores the increased field value before the dynamic concatenation.
The dynamic argument is read from the field and the returned string is stored
in `last`. The later decrement changes only the integer field. The reference
execution observes `last` as `value=13` after `update` returns `11`.

The independently authored `CustomBootstrap` delegates with the JVM's
lookup, call-site name and type, recipe and constants. The modified class
points at that bootstrap in its `BootstrapMethods` attribute. `javap`
shows the same recipe at the custom static handle. Running the modified
class in OpenJDK passed the same reference paths and effects as the original.

The ordinary JVM `get-class` result for the standard fixture preserved the
constructor, arithmetic, fields, call, branches, loop, typed catch, string
concatenation and switch. The rendered class compiled in a scratch directory
and passed all `Check.java` paths. For the opaque variant, `get-method`
kept the initial `value` store and the `last` assignment with its typed,
unavailable result. It retained the original bootstrap owner, signature and
input value at byte range `[15, 20)`, followed by the decrement and return.
That gap accounts for the only issue in the selected method.

For the DEX comparison, D8 8.9.35 converted the same compiled class using
debug mode and min API 28. The ordinary DEX `get-class` output expressed all
four effects of `update` in the same order. That output also preserved the
arithmetic, branch, loop and exception paths above. Recompiling that output
in a test scratch directory and running the same `Check.java` passed every
reference case.

## Overlapping exception handlers

`ExceptionChains.guarded` reads the array before the division. Only the
arithmetic failure enters the inner typed handler. Its `mark(2)` precedes
the shared `mark(4)`. An index failure takes the outer handler directly,
and null escapes both handlers before any mark. `guardedFinally` uses
`mark(3)` on success, after the inner arithmetic handler, and before an
unmatched exception resumes its propagation. An index failure then reaches
the outer handler and calls `mark(5)`. `CheckHandlers.java` ran all eight
paths and checked their exact ordered traces.

The equivalent D8 input produced both methods with no recovery issues.
The recovered DEX source held the same typed catches and cleanup effects.
Running `CheckHandlers.java` on that source also passed all eight paths.

The final JVM `get-class` output preserves both typed catches and all
cleanup copies. Successful division reaches one `mark(3)` before returning
the quotient. The arithmetic handler calls `mark(2)` and then `mark(3)`
before returning `-7`. The catch-all executes `mark(3)` and rethrows the
original exception. The outer typed catch then calls `mark(5)` for an index
failure. A null input escapes after cleanup. The rendered `mark` method
preserves the multiply and add that encode the order of those calls.
The source recompiles and passes all eight reference paths with their
original trace values.
## Overload binding review

The original wide route evaluates `once` before either call or allocation.
Its selected static and instance calls, direct construction, and delegated
construction all use the integer formals. The narrow path intentionally
keeps the character formals. Each implementation records a distinct digit
and uses its argument once in the return value or field assignment.

Diana 0.4.6 publishes the character argument bare at the integer call
sites. Recompilation changes the selected implementations, so both the
value and the trace change while `once` still runs exactly once. Diana
0.4.7 emits the integer view at each widening site. The allocation,
delegation, call and effect sequence then matches the original for the
ASCII and high UTF-16 code units. The narrow route and its trace are
preserved. Call attribution and execution run separately from this review.

The generic control selects the overload with one shared `T`. The
`List<String>` argument fixes `T` to `String`. The alternative with
`U extends Number` cannot accept that list, and the two-`Object` overload
is less specific. The candidate preserves the `String` argument type and
returns 1, as does the original. An erased cast to `Object` would change
the call to the fallback and return 3.
