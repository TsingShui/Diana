# Reentrant initialization of interface fields

This independently authored DEX case applies the same mechanism as the
authored JVM interface control. Two external getters read public static
final fields with no encoded values. An interface initializer assigns
the first field, calls a foreign effect, and then commits the second
field. The Java effect model reenters two original interface getters.
At this execution point the first field is visible and the second
retains its default value.

| Original selector | Role |
| --- | --- |
| `e0:l0:c2:m3::Lfixture/RuntimeFace;-><clinit>()V` | Stores FIRST at code unit 2, calls the external effect at 4, stores SECOND at 8. |
| `e0:l0:c0:m0::Lfixture/FaceConsumer;->first()Ljava/lang/String;` | Reads FIRST directly and triggers original initialization. |
| `e0:l0:c0:m1::Lfixture/FaceConsumer;->second()Ljava/lang/String;` | Reads SECOND directly and triggers original initialization. |
| `e0:l0:c2:m4::Lfixture/RuntimeFace;->readFirst()Ljava/lang/String;` | Reads FIRST during the callback and when initialization has completed. |
| `e0:l0:c2:m5::Lfixture/RuntimeFace;->readSecond()Ljava/lang/String;` | Reads SECOND during the callback and when initialization has completed. |

The [prepared DEX](classes.dex) was assembled from the three Smali
files with APKTool 2.11.1 and API 24 using `SmaliBuilder.buildFile` and
`DexBuilder(new Opcodes(24, -1)).writeTo`. Its SHA-256 is
`73764c40ac9c7726a5651101264c580d34c7c9cafe110dc218500c09eb225c08`.

The [original replay](raw_proof.py) independently decodes field
declarations, exact method locations and instruction operands. It
models a synchronous native callback with two same-thread interface
reads. The [Java control](RuntimeFace.java) uses an effect-free
`literal()` helper so Javac retains an active read of FIRST. With either
field read first, the [authored trace](scenarios.json) records the
callback observing `first:null` before the selected field read returns.
The Java observer and effect model remain the same when compiling the
complete, exact interface and consumer publications together.

The frozen 0.4.13 executable passes the SECOND-first control and fails
the FIRST-first scenario. FIRST appears as a Java constant, and the
compiled consumer returns it before the callback. The 0.4.14 candidate
passes both cases and all 14 DecBench checks. The original raw and Java
controls pass both scenarios.

Run the [default suite](decbench.json) from the repository root.

```sh
python3 -B quality/decbench run --suite dex-interface-runtime-reentry --binary /path/to/diana
```
