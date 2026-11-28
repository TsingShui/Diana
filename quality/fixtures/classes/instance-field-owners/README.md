# Instance-field owners under field hiding

The independently authored Smali defines three classes with an instance
field named `value` of type int. `ShadowFields` extends `ParentFields`,
which extends `RootFields`. The [observer](CheckFieldOwners.java)
uses distinct initial values for every field slot and snapshots all
slots of both objects after each selected operation.

| Original selector | Method | Original field operation |
| --- | --- | --- |
| `e0:l0:c3:m6` | `Lfixture/ShadowFields;->readOwn()I` | Read the immediate receiver's own slot |
| `e0:l0:c3:m7` | `Lfixture/ShadowFields;->readParent()I` | Read the parent slot on the current receiver |
| `e0:l0:c3:m8` | `Lfixture/ShadowFields;->readRoot()I` | Read the root slot past the immediate superclass |
| `e0:l0:c3:m9` | `Lfixture/ShadowFields;->writeParent(I)V` | Write the parent slot on the current receiver |
| `e0:l0:c3:m10` | `Lfixture/ShadowFields;->writePeerRoot(Lfixture/ShadowFields;I)I` | Write the root slot on a supplied receiver, then read the current receiver's root slot |
| `e0:l0:c3:m11` | `Lfixture/ShadowFields;->writeRoot(I)V` | Write the root slot on the current receiver |
| `e0:l0:c0:m1` | `Lfixture/ForeignFieldAccess;->readRootThroughShadow(Lfixture/ShadowFields;)I` | Read the root slot from outside the hierarchy |
| `e0:l0:c0:m2` | `Lfixture/ForeignFieldAccess;->writeChosenRoot(Lfixture/ShadowFields;I)V` | Call the receiver producer once, then write the root slot on its result |

The [original DEX witness](raw_proof.py) resolves the superclass
chain, explicit field-reference owners and instruction operands. It
checks the receiver producer's static counter update and its return
operand in the prepared DEX. The peer scenarios pass a distinct object,
the current receiver, or null. The receiver-producer scenarios check
that the counter increases once before a normal write or the original
null failure.

The [default DecBench suite](decbench.json) recompiles all four
complete, exact class publications together with the independent
observer. The [12 authored traces](scenarios.json) agree with both the
original DEX witness and the authored Java execution. Frozen
Diana 0.4.17 chooses the most-derived field when the original refers to
a hidden parent or root slot, with 28 passing and eight failing checks.
The repaired engine keeps the original owners, evaluates the receiver
producer once, and passes all 36 checks. The comparison recovered eight
checks with the same coverage and no new failures.

The DEX was assembled using APKTool 2.11.1 and API 24 through
`SmaliBuilder.buildFile` and
`DexBuilder(new Opcodes(24, -1)).writeTo`. Its SHA-256 is
`a541c767b61d1400241b5fbf9c554c2a71c14808e04cb13e52ac4c1f5ae780f5`.

```sh
mkdir -p .cache/decbench
fixture=quality/fixtures/classes/instance-field-owners
PYTHONPATH=quality/decbench python3 -B "$fixture/raw_proof.py" \
    --output .cache/decbench/instance-field-owners-original.json
python3 -B quality/decbench run --suite dex-instance-field-owners \
    --binary /path/to/diana --output .cache/decbench/instance-field-owners.json
cargo nextest run -p diana-decomp --test instance_field_owners_test
```
