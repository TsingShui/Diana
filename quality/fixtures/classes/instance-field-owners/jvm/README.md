# JVM hidden instance fields

These sources are independently authored under the [repository license](../../../../../LICENSE).
`FieldParent` and `FieldChild` each declare an `int slot`. Javac selects the
parent field for `super.slot` even though the receiver is a `FieldChild`.
The unrelated `FieldPeer` class selects the same field through an upcast
of an effectful receiver expression. Its bytecode invokes the receiver
helper once and then accesses the parent field without a `checkcast`.

| Original member | Selected operation |
| --- | --- |
| `LFieldChild;->readParent()I` (`j0:m1`) | `getfield FieldParent.slot:I` at byte 1 |
| `LFieldChild;->writeParent(I)V` (`j0:m2`) | `putfield FieldParent.slot:I` at byte 2 |
| `LFieldChild;->writeThenReadBoth(I)I` (`j0:m3`) | Parent write, child read and write, parent read, child read |
| `LFieldChild;->dispatch()I` (`j0:m4`) | `getfield FieldChild.slot:I` |
| `LFieldChild;->callParent()I` (`j0:m5`) | `invokespecial FieldParent.dispatch:()I` |
| `LFieldPeer;->readPeer(LFieldChild;)I` (`j0:m2`) | One receiver call, then `getfield FieldParent.slot:I` |
| `LFieldPeer;->writePeer(LFieldChild;I)V` (`j0:m3`) | One receiver call, then `putfield FieldParent.slot:I` |

[The independent witness](CheckFieldOwners.java) starts with slots `37,5`.
A parent write to 49 must leave the child slot at 5. A mixed update sets
only the parent slot to 11 and increments the child slot to 7. The peer
write to 66 changes a separate object's parent slot. A null peer must throw
`NullPointerException` after exactly one receiver evaluation. The direct
superclass and virtual dispatch values on a fresh object are 137 and 205.

On OpenJDK 21.0.12.1 the original passes all eight runtime and seven bytecode
binding checks. Frozen Diana 0.4.17 rebinds both parent and peer accesses to
`FieldChild.slot` when its exact class publications are recompiled. It passes
three runtime and two binding checks. The repair's debug build passes all
15 checks. [Semantic review](review.md)

```sh
fixture=quality/fixtures/classes/instance-field-owners/jvm
python3 -B "$fixture/verify.py" --binary /path/to/diana \
  --output .cache/fixtures/instance-field-owners/jvm/report.json
cargo nextest run --locked -p diana -E 'test(=recovered_jvm_instance_field_accesses_preserve_declaring_owner)'
```

Python 3 and JDK 17 or later suffice. The verifier compiles the parent and
runner once and runs them against both the original classes and the exact
Diana publications. It saves the executions, `javap` listings, exact source
and binary digests. Omit `--binary` to check the original alone.
