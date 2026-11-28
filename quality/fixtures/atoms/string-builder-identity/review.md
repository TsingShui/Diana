# Semantic review

Each original uses a builder or buffer and observes its nonempty
result. The recording hook compares that reference with a string
interned before the method call. The actual Java source and retained
DEX model produce the expected text and a distinct reference in
each scenario.

Diana 0.4.12 emits source constant concatenation in four cases. The
int/char/boolean chain and the two-string chain produce the same
interned references when compiled. The recovered arithmetic operand
is the correct signed result -127. Its emitted literal participates in
the same constant concatenation. The recovered static field is
declared final and initialized with a String literal. Java treats
its use in the concatenation as a constant expression. The four cases
retain the correct text and change the reference comparison.

The candidate keeps literal-only construction and append calls. The
arithmetic case still appends the signed int value -127. For the static
field case, the leading String operand passes through Object and back to
String. The casts preserve that value and its evaluation order. The
source concatenation therefore creates a runtime string even when the
field becomes a Java constant variable.

In the JVM companion, the actual compiler-generated string literals
retain their interned identity. The original runtime concatenations,
including the operations that read a field initialized in `<clinit>`,
produce new references in the recompiled candidate. The two-call checks
observe that distinction independently of string content. The companion
checks methods and fields within one class.

Both dynamic subjects retain their recorded signed value or branch
and produce a distinct string in the exact recovered Java. The
Object-append subject retains the builder call in 0.4.12 and likewise
preserves the observed result. The raw witness verifies selected
operand and field encodings within the original retained scope and
uses explicit library models. Independent Java execution verifies
the actual JDK reference behavior.
