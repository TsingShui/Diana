# Semantic review

The original payloads describe a write to the existing primitive array.
The receiver is evaluated once before the instruction, and the
instruction checks the complete destination extent before writing any
prefix. An empty payload still observes a null destination. The
untouched tail and original alias remain visible in successful fills.

In 0.4.18 each emitted store evaluates its receiver independently. When
the receiver callback succeeds, the trace records two calls instead of
one. A short destination receives the first element before the later
store fails. The protected bounds handler then reports 11 instead of
the initial 7. The empty fill returns normally with a null receiver.
The raw floating-point observer also finds that the reconstructed
noncanonical quiet NaN loses its original payload bits.

The raw replay and Java reference agree on the supplied receiver
exception. The effect precedes the exception, its typed handler
records the exact object, and the destination remains untouched.
The normal reference paths retain the original signed zeros and
the noncanonical quiet NaN in the array.

The repaired publication captures the effectful receiver once. It
checks the full length before the first store and keeps both operations
inside the original protected region. The short handler now sees 7,
the empty null destination throws, and the float check sees both
original signed zeros and the exact NaN bits. The full recompiled
publication passes all 14 authored scenarios.

The existing ValueOccurrences, RefinedCarriers and ValueOccurrenceEffects
fixtures still publish compact byte-array initializers. Their adjacent
literal-size allocations establish the same array binding and enough length
for every recovered write. The two-array case keeps the allocations distinct.
