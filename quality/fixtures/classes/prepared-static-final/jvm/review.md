# JVM prepared static final review

`DefaultChild` declares `PREPARED_ZERO`, `PREPARED_NULL`, `PREPARED_BYTE`
and `PREPARED_SHORT` without constant attributes or executable writes.
`ReadDefaultChild` reads them with `getstatic`. The first such read begins
initializing `DefaultChild`, which runs the superclass's initializer before
returning the field's prepared value. The parent either records `start` and
`complete` or records `start` and `fault` and throws the authored cause.

Diana 0.4.16 keeps the symbolic access in the recovered reader but renders the
child's prepared primitive fields with literal initializers. Javac resolves the
integer, byte and short reads to literals and adds their `ConstantValue`
attributes. A zero-first read returns before the parent's observable effects.
A zero-first read paired with a throwing parent still returns zero while the
next reference read receives the first initialization exception. When the
reference read fails first, the following zero read returns normally instead
of observing the class's erroneous state. The genuine integer and string
constant reads retain their compiled behavior and interned reference.

Diana 0.4.17 publishes the prepared primitive values with an effect-free
nonconstant conditional, with the declared primitive cast where needed.
The rebuilt readers retain `getstatic` on the exact declaring class and
member. The normal and exceptional superclass traces match the original,
including cause identity and the erroneous-class outcome. Both narrow reads
retain their field descriptors and return zero after successful initialization.
The genuine integer and string constants still compile to passive literal
reads. The rebuilt child contains new default-valued writes from javac, while
the generator's original class contains no static writes.
