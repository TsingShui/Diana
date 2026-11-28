# Signed-zero selection review

The original float branches write distinct raw values to the same join
register. True contributes `80000000` and false contributes `00000000`.
The double branches use the opposite polarity, with true contributing
positive zero. A raw-bit conversion returns those carriers exactly. On
the field-write paths the same carriers reach the original float field and
the original double field. The direct-return float control retains both
possible values without joining them.

The 0.4.20 publication collapsed the joined float values to positive zero
and the joined double values to negative zero. The losing arms return the
wrong raw bits and store the wrong field bits. In the protected call,
the negative arm passes positive zero to the helper. The helper receives
the wrong bits on both its success and fault paths. The typed catch
retains the original thrown exception identity, but observes the helper's
incorrect write. The other arm and the direct-return control retain their
established behavior.

In the candidate publication, the selected raw sign reaches every checked
post-join consumer. Both protected helper outcomes store the correct raw
value before returning or throwing. The normal observer sees no caught
exception. The failing observer sees the same supplied exception and the
field value written before it. Both signed-zero controls with separate
returns agree with their original cases.

The bounded original DEX replay and exact recovered-class execution checked
these paths independently. The replay models the two standard bit
conversion calls on the selected zero values. Native Android execution and
unrelated floating-point values are outside this probe's coverage.
