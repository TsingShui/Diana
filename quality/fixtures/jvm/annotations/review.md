# Semantic review

The class declaration at `j0` carries both `Rich` and `Quiet`. Its
`Rich.text` element points to a modified UTF-8 string constant rather
than to a `CONSTANT_String` slot. The remaining elements independently
establish `Mode.BETA`, `int[].class`, an array containing 1, -2 and 3 in
that order, `Nested.label`, the original long value and true. The shared
Java renderer displays all these values and the invisible `Quiet`
declarations. Only `PREPARED` at `j0:f0` obtains its string from a
static field ConstantValue. In xref, that origin stays distinct from
annotation fields and direct instructions.

`calculate` narrows the long argument to int, adds the original second
argument and reads the receiver's saved field to form its return value.
Both parameter annotations are rendered next to their original
parameters. The JVM annotation table reports parameter ordinals, so the
wide first parameter does not shift the second annotation to ordinal 2.
The malformed variant lacks only the visible class annotation string.
The emitted source still displays independent annotations and values and
the method body, and the reference query still finds the nested annotation
value with a local issue for the unreadable original string.

The original enum constructor has descriptor `(Ljava/lang/String;II)V`.
Its parameter metadata marks the generated name and ordinal as synthetic.
The parameter-annotation table has one real position at encoded index 0,
aligned to descriptor ordinal 2 only when that metadata is visible. The
unaligned input still exposes the same literal and encoded position,
without claiming it belongs to a specific descriptor slot.

The `Nested.label` default and `TypeUsage.value` return type annotation
preserve their encoded values and original declaration contexts in xref.
Source recovery retains the method declarations and marks their two
unsupported placements visibly. The review does not infer Java semantics
for a malformed annotation value or unaligned parameter slot.
