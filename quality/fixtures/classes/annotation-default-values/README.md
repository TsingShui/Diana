# DEX annotation element defaults

This independently authored fixture covers a DEX annotation interface and a
sibling annotation type. The class annotation directory for
`Lfixture/DexDefaults;` contains a system-visible
`Ldalvik/annotation/AnnotationDefault;` item. Its only element is `value`, a
nested annotation of type `Lfixture/DexDefaults;`. The named elements in that
payload supply defaults for methods on the same interface. An absent name
means that the method requires a value. [Android DEX format](https://source.android.com/docs/core/runtime/dex-format#annotationdefault)

| Original member in `Lfixture/DexDefaults;` | Locator | Observed default |
| --- | --- | --- |
| `classChoice()Ljava/lang/Class;` | `e0:l0:c0:m0` | `String[].class` |
| `count()I` | `e0:l0:c0:m1` | `-33` |
| `empty()[I` | `e0:l0:c0:m2` | An empty integer array |
| `enabled()Z` | `e0:l0:c0:m3` | `true` |
| `label()Lfixture/DexLabel;` | `e0:l0:c0:m4` | `@DexLabel(rank = 4, value = "west")` |
| `labels()[Lfixture/DexLabel;` | `e0:l0:c0:m5` | The annotation `(-1, "north")` followed by `(5, "east")` |
| `name()Ljava/lang/String;` | `e0:l0:c0:m6` | `row`, a line feed, and `"blue"` |
| `policy()Ljava/lang/annotation/RetentionPolicy;` | `e0:l0:c0:m7` | `RetentionPolicy.RUNTIME` |
| `required()I` | `e0:l0:c0:m8` | Absent |
| `scores()[I` | `e0:l0:c0:m9` | `[3, -2, 3]` in that order |

The sibling `Lfixture/DexLabel;` has no default carrier. Its `rank()I` at
`e0:l0:c1:m10` and `value()Ljava/lang/String;` at `e0:l0:c1:m11` both require
values. The ordered array of nested annotations checks recursion and original
class identity. `empty()` and `required()` distinguish an encoded empty array
from the absence of a default. The [JVM fixture](jvm/) exercises the sibling
input format.

## Preparation and checks

Apktool 2.11.1's embedded Smali assembler generated this DEX at API 24 through
a local Java adapter that accepts a Smali directory, output file and API.
The prepared DEX has SHA-256
`ef45b9104d4f58d9c406f21389b514995f0393a9656451dbd35a2b32689603fb`.

```sh
fixture=quality/fixtures/classes/annotation-default-values
work=.cache/fixtures/annotation-default-values
mkdir -p "$work/smali"
cp "$fixture"/*.smali "$work/smali/"
java -cp "$APKTOOL_JAR" "$SMALI_ADAPTER" "$work/smali" "$work/classes.dex" 24
cp "$work/classes.dex" "$fixture/classes.dex"
```

The bounded raw DEX reader checks the original members, carrier visibility,
nested declaring type, element names, and encoded values. An independent JVM
observer calls `Method.getDefaultValue` on both authored annotations and on the
exact Diana class publications compiled separately. It compares the return
and annotation descriptors, enum owners, values, and array order against the
[scenarios](scenarios.json).

The probe also creates a scratch DEX by replacing the encoded integer for
`count()` with a same-width reference to the original string `west`. Class
analysis and selection of `count()` must report a metadata issue on original
method ID 1. Selection of `labels()` must succeed with an empty issue list.
Reflection on the class publication and the selected `labels()` shell must
show an absent default for `count()` and retain every valid sibling default.

```sh
python3 -B quality/decbench run --binary /path/to/diana \
  --suite dex-annotation-default-values
```

On frozen Diana 0.4.22 all nine encoded defaults are absent from the
publication and the malformed value goes unreported. The three declarations
requiring a value pass. The original DEX and the authored Java agree on all
twelve observations. The repaired build passes those twelve observations,
the malformed scope case, and all 49 suite checks. [Semantic review](review.md)
