# Semantic review

The list-form DEX call uses registers p2 and p3 for the long and p4 for
the trailing int. The first recovered invocation uses the incoming
`long` and `int` once each. The range-form invocation starts at p0 and
spans p0 through p4. The second invocation reads the initial double,
the same long, and the same int in that order. Each call's return value
is saved separately.

The next instruction sequence copies the handle receiver to v2 and
the long to v3/v4 before invoking the original `invokeExact` method.
The additional prototype specifies one long argument and an int
result. The recovered call uses the original handle and passes the
long, leaving out its high register word. The ordinary class output
saves that distinct result and passes it together with the first value
to `finish`. Its array argument contains the first, second and third
call results in the order established by the original filled array.

The executable comparison models the ordinary and signature-polymorphic
hooks at these checked argument and result types. For every call it
records the argument values, including both words of the double and
long through their full values, and all array elements at the final call.
An exception at any of the four calls must prevent every later effect.
The model covers the checked `(J)I` invocation and leaves dynamic JVM
handle adaptation and linkage unreviewed.

The malformed cases exercise independent facts. With the range word
count reduced to four, invalid grouping preserves all four encoded
arguments and the failure belongs to type recovery. An out-of-table
method at PC 0 precedes a later broken prototype. With both
polymorphic slots broken, the error identifies the method slot at PC
10. With only its second slot broken, the error identifies the
prototype there. The three native siblings remain visible for each
damaged copy.

Both ordinary Engine regression tests passed. The new fixture and its
four damaged copies produced byte-identical ordinary JSON results and
diagnostics before and after the binding change. Seven prior fixture
classes also matched, including a wide custom invoke with bootstrap
arguments. In the typed hook model, 512 double/long bit patterns with
five possible failure placements yielded 2,560 matching results, call
traces and escaping exception identities. A separate comparison of
22,352 register-count, receiver, width and prototype-length combinations
found the same surviving register words or first missing-argument error
in the old and new grouping algorithms.
