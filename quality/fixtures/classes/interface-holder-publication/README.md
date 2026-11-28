# Interface field publication during initialization

This independently authored fixture models a same-thread call into an
interface while its initializer is running. The observer starts with `seen`
unset. Both interfaces allocate one object and write the same reference to
their two blank static final fields.

| Original member | Locator | Ordered behavior |
| --- | --- | --- |
| `Lfixture/PublicationFace;-><clinit>()V` | `e0:l0:c2:m1` | Allocate once, publish `FIRST` at code unit 5, call `observe()` at 7, and publish `SECOND` at 10. |
| `Lfixture/PublicationFace;->observe()V` | `e0:l0:c2:m2` | Read the interface's `FIRST` at code unit 0 and write the same reference to `HolderObservation.seen` at 2. |
| `Lfixture/HolderTail;-><clinit>()V` | `e0:l0:c1:m0` | Allocate once and publish the two aliases consecutively at code units 5 and 7. |

The observer must record the non-null object that becomes both public fields.
In the tail control, no callback can observe the intermediate writes. The
[Java explanation](PublicationFace.java) uses a field initializer helper to
carry the same value into `SECOND`. The Smali input keeps that value in one
local across the call and supplies the original field and method identities.

## Preparation and checks

Apktool 2.11.1's embedded Smali assembler prepared the DEX at API 24 via
`SmaliBuilder.buildFile` and `DexBuilder(new Opcodes(24, -1)).writeTo`.
The prepared [classes.dex](classes.dex) SHA-256 is
`07d92b7ac895dbf018144f07aff627ef20c6ffa89a24ac226f19dbe46cd4b0cc`.

```sh
fixture=quality/fixtures/classes/interface-holder-publication
work=.cache/fixtures/interface-holder-publication
mkdir -p "$work"
java -cp "$APKTOOL_JAR" "$SMALI_ADAPTER" "$fixture" "$work/classes.dex" 24
cp "$work/classes.dex" "$fixture/classes.dex"
```

The checked instruction sequence and exact DEX targets were read separately
from the rendered text. On frozen Diana 0.4.25, the synthetic holder delayed
public `FIRST` until after its callback. The authored Java and exact baseline
class publications were compiled and run in separate Java 21 processes. The
callback observed `FIRST` in the authored control and observed `null` in the
baseline. Both runs returned equal field aliases after initialization, and the
tail control agreed.

The 0.4.26 debug publication was checked through ordinary `get-class`. Its
initializer writes the interface's `FIRST`, calls `observe`, and writes
`SECOND` from the same saved reference. The tail control is unchanged. The
initializer fallback is Java-like source. Its verification checks the
published schedule and does not require Java recompilation or a device run.
[Semantic review](review.md)

```sh
diana get-class "$fixture/classes.dex" fixture.PublicationFace --format json
cargo nextest run --locked -p diana-decomp --test interface_holder_publication_test
```
