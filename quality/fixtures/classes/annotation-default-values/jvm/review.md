# JVM annotation defaults review

The original class-file attributes cover eight primitive types, escaped text,
a class literal, an enum, three kinds of arrays and nested annotation values.
`javap` independently identifies an `AnnotationDefault` attribute on every
method that has an authored default. The compiled annotation application uses
the defaulted scalar number and the nested annotation array at runtime.

The 0.4.22 source adapter leaves the original methods and `@Retention`
annotation visible but drops all 16 default values. The available output
compiles. Reflective default access then returns absent for every element,
and the unchanged annotation application fails at its first defaulted member
with `IncompleteAnnotationException`. The required element still correctly
has no default. The raw reference query retains the `Instant` class-literal
origin and offset independently of the source declaration.

The duplicate and trailing-byte cases each damage only the integer default.
The wrong-type case also affects that method. The encoded value itself is
a valid long, while the method declares an integer result.
The invalid constant-pool index damages the name of the nested annotation's
explicit `value` element. That partial nested value cannot provide the
original default. Each negative test checks the affected method's diagnostic
and verifies that unaffected scalar, array and nested defaults remain usable.

The repaired publication emits the typed defaults on their original methods
and imports the default-only `java.time.Instant` type. Recompilation and the
unchanged observer recover all 18 ordinary results, including the annotation
application. Each damaged variant reports one malformed method and retains
every independent value on recompilation. The malformed nested raw xref
still includes its checked type at file byte 1174, and the valid class
literal reference remains at byte 1148 in this build of the fixture.
