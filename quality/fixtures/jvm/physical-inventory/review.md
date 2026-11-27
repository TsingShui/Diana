# Review of JVM declaration and instruction facts

This review covers the independently authored class files from the fixture
recipe. Python 3.13.14 prepared the archive. OpenJDK 21.0.12.1 `javap`
confirmed the instruction positions in the standalone base class.

The base and versioned definitions declare the same class, members and
signatures. Their string constants differ before the same NUL and U+1F331
code points. The reference and method listings agree with the ClassFile
table order. The class stored as `misleading/NamedFromPath.class`
declares `fixture/jvm/Helper` in its own constant pool. The query preserves
that name and its physical location.

In `forward(I)I`, the load at offset 0 supplies the branch at offset 1.
Zero selects the block at offset 9 and returns zero. The other path passes
the argument to `Helper.twice(I)I` at offset 5 and returns the helper's
result. The helper shifts the signed 32-bit argument left by one.
The `unused()V` pool entry describes no instruction in either definition.
The constructor writes three after the superclass call. A later `read()I`
loads that value from the same field. The separate `forward(J)J` overload
returns its argument with the original width.

An independent JVM execution of the generated class files confirmed positive,
negative and zero inputs to the integer overload. It also confirmed the wide
argument, the constructor field, the NUL and supplementary string units, and
the original two-slot constants, including negative zero. This execution uses
fixture classes placed at their declared binary names for the reference JVM.
The archive query exercises the intentionally mismatched entry name.

The damaged Code variant still gives the same reference at offset 5.
The undefined byte at offset 11 prevents reading farther in that method.

The nested method loads its input at offset 0, calls the exact outer static
method at offset 1, and returns that result at offset 4. The callee adds one
with 32-bit JVM integer arithmetic and returns without making another call.
Running the generated class files in OpenJDK confirmed the positive, negative
and signed overflow cases. The reference result locates the instruction at
`j1/j0:m0`. The depth-one outline follows it to `j2:m0`. The broken catalog
row leaves that same visible definition available as a candidate while the
set of all nested definitions remains uncertain.

The base `Main` constructor invokes its superclass constructor before the
field write. `Base` passes the same receiver to `java/lang/Object.<init>()V`
before returning. The depth-two outline follows the `Base` declaration and
retains the bytecode's `Object` constructor symbol. The fixture never supplies
an `Object` declaration, so that symbol has no input locator.

In the dynamic fixture, `ldc_w` and `invokedynamic` use bootstrap rows
0 and 1 respectively. Both instruction offsets are zero. The load's
row has the string and two occurrences of dynamic constant `nested`.
The invocation's row has one occurrence of `nested`. That constant's
row 2 supplies the string to `seed`. The reference queries preserve the
load's direct string argument, the nested row's separate string argument,
and the original method coordinates for both sites. OpenJDK `javap`
independently confirmed these instruction and bootstrap table entries.
The fixture exposes the symbolic topology. Runtime execution of the
bootstrap methods requires an external declaration and remains unknown.

The damaged dynamic load replaces its bootstrap method handle with an
ordinary method reference. Its bootstrap arguments still identify row 2,
and the invocation's bootstrap row remains valid. The diagnostic points
to the load instruction while the `seed` references at both original
instruction offsets remain available.
