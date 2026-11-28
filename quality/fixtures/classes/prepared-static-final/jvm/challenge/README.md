# Active reads through an inherited field owner

This independently authored JVM fixture extends the prepared-static-final
example with an observable superclass and a symbolic field reference
through a subclass. [prepare.py](prepare.py) emits
`DefaultHierarchyOwner.class` with two prepared-only static final
fields and two genuine ConstantValue fields. The original provider
contains one instance constructor and no initializer or static
writes. Javac compiles the consumer, subclass and independent observer.

`DefaultHierarchyOwner` extends `ObservableRoot`, whose initializer
records start and completion or throws the supplied root fault.
`HierarchyAlias` extends `DefaultHierarchyOwner`. Its distinct
initializer records a start, calls the inherited reader while the
alias is initializing, and then either assigns 5 or throws its own
supplied fault.

| Original method in `LReadStaticHierarchy;` | Bytecode at offset 0 | Resolved declaring class |
| --- | --- | --- |
| `j0:m1 inheritedLong()J` | `getstatic HierarchyAlias.PREPARED_LONG:J` | `DefaultHierarchyOwner` |
| `j0:m2 directNull()Ljava/lang/String;` | `getstatic DefaultHierarchyOwner.PREPARED_NULL:Ljava/lang/String;` | `DefaultHierarchyOwner` |
| `j0:m3 activeAlias()I` | `getstatic HierarchyAlias.ALIAS_VALUE:I` | `HierarchyAlias` |
| `j0:m4 compiledLong()J` | `lconst_0` | Literal from a genuine ConstantValue |
| `j0:m5 compiledText()Ljava/lang/String;` | `ldc "hierarchy"` | Literal from a genuine ConstantValue |

The inherited read initializes the field's declaring class and its
observable superclass. An explicit read of the alias's own field
initializes the alias. If the alias fails after the declaring class
completes, the inherited read still succeeds through that symbolic
alias and the actual alias field reports an erroneous class on retry.
A root failure prevents both subsequent active reads of the declaring
class and active alias initialization. Each first initializer failure
must expose the exact authored cause. The genuine constants continue
to return their defaults and interned text without active use.

The [six authored traces](scenarios.json) run in fresh JVMs. The
[verifier](verify.py) checks original declarations, reference operands,
instruction order and literal paths in the actual classfiles using
Javap. It recompiles the exact complete Diana publications of the
provider, alias and consumer with the same authored superclass and
observer. The receipt records both runtime traces and bytecode facts.

Frozen Diana 0.4.16 matches four original traces and fails two.
Turning the prepared long into a Java constant defers root
initialization when the inherited read comes first. In the root-fault
scenario it also returns zero on retry through the alias. The repaired
engine preserves the three active reads and the two genuine constants
in the recompiled classfiles, and all six traces match.

Run from the repository root using Python 3.11 and a JDK.

```sh
python3 -B quality/fixtures/classes/prepared-static-final/jvm/challenge/verify.py \
    --binary /path/to/diana --output .cache/decbench/prepared-static-challenge.json
```

The verifier puts generated classfiles, disassembly and full
publications beside the specified ignored report. Use `--classes` with
an existing original evidence directory to reuse exact prepared
inputs across runs.
