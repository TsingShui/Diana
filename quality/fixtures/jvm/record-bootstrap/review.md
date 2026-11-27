# JVM record bootstrap review

The original class file has an ordered `Record` component table for
`count:I`, `reading:F`, `label:Ljava/lang/String;` and `values:[I`.
Independent `javap` output shows the same order in the shared bootstrap row.
Its four method handles all read the actual private final instance fields.

The original equality path passes the receiver and the original comparison
object to the bootstrap-linked operation. The recovered operation retains both
operands. Hash and string operations retain just the receiver. All three
operations keep the exact typed field references and each original method
identity. A renderer that substituted a generic array hash or called these
same methods on the receiver would lose the original bootstrap behavior.
The displayed operation represents the bootstrap contract without claiming
a platform-dependent hash combination or formatting algorithm.

The executed fixture checks canonical NaN equality and compatible hash codes,
distinguishes signed zero and distinct arrays, retains null and mixed-class
checks, and confirms the independent increment method. The empty record tests
a zero-length component list through the original JVM and source publication.

The contradictory string changes one displayed component name while keeping
the original `Record` declaration and getter. This disagrees with the
recognized record shape. The other variant leaves the field declaration
readable but refers to pool entry zero in the getter. Their generated
operations remain locally unavailable, with the original byte range recorded
for each. The independent arithmetic method remains readable. The two
damaged variants were inspected through the CLI rather than executed.
