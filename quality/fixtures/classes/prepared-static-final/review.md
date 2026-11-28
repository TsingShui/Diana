# Preparation and initialization review

The encoded array contains integer zero and nine for the first two fields.
The trailing integer and String have descriptor defaults without encoded
entries. The child declares no initializer. Its parent initializer calls the
two recording hooks in order, and the original reader methods use static
field reads. The encoded control returns a literal zero without a field read.

Diana 0.4.16 preserved the original field identities in the reader. It
published both integers with zero constant initializers as class
declarations, which makes the recompiled consumer skip initialization when
reading the preparation-only integer. In the normal integer-first trace,
the first zero appears before `start`. In the fault integer-first trace it
also escapes the modeled parent exception. In the null-first fault trace,
the null read throws the initialization error and the later integer read
returns zero instead of observing the erroneous class.

The candidate retains the literal zero for the encoded control and renders
the implicit zero with a nonconstant expression. Its only selected branch
evaluates a literal zero and has no observable effects. The recovered
consumer retains static reads. Both start and finish occur before the first
value. When finish throws, the first read reports the exact original cause
and the second reports the erroneous class. The literal control has no
initialization effects in any trace.

These observations cover the authored normal and exceptional paths for the
selected classes and the native hook model. Native execution of the DEX on
Android was outside the probe. The ordered Java traces and original DEX
structure were checked separately.
