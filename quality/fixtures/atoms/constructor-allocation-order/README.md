# Constructor preallocation order

This independently authored fixture extends the preallocation order issue
observed in FairEmail 1.2338 `HtmlHelper.autoLink`. The app computes a `Pattern`
before allocating the visitor. This fixture puts two calls before allocation,
adds observable class initialization, and exercises faults and retries. The
fixture sources contain original material under the repository license.

`LConstructorPrefix;->run(IZ)I` is `e0:l0:c0:m4` in the prepared `classes.dex`.
The Smali sources define the input. The Java explanation provides corresponding
helper implementations and a driver that runs each scenario in a fresh JVM.
The driver is absent from the DEX.

## Original behavior

The DEX calls `prepareFirst` at `0x0` and `prepareSecond` at `0x4`. It allocates
`PrefixValue` at `0x8`, adds one to the prepared value at `0xa`, and doubles the
original input at `0xc`. The initializer `LPrefixValue;-><init>(II)V` at `0xe`
receives both values. The method reads the initialized field at `0x11` and
returns it at `0x13`. The method has no local exception table.

For input `i`, the preparations log `i` and `i + 1` in that order. The second
call throws when the fault flag is true. A successful preparation returns
`i + 3`. The constructor logs the arguments `i + 4` and `2 * i`, and the returned
field holds ten times the first argument plus the second. The constructor
throws after its log when the second argument is 14.

[The independent scenario manifest](scenarios.json) covers cold and warm
success, preparation failures with retries, first class initialization failure
followed by a retry on the erroneous class, and constructor failure followed
by a warm retry. Cold preparation failure leaves the allocated class cold.

## Preparation and replay

The prepared DEX was assembled at API 24 with Apktool 2.11.1 and OpenJDK
21.0.12.1. Its SHA-256 is
`6ebfefce2a7cc6e20d1198a786e263170da396dabaecc9faa62bdac444d34965`.
The assembly driver is the locally available wrapper around the Apktool Smali
assembler. Replay uses the checked-in DEX and requires no assembler.

```sh
fixture=quality/fixtures/atoms/constructor-allocation-order
prepared=.cache/fixtures/constructor-allocation-order
mkdir -p "$prepared"
java -cp .cache/decbench-repair/smali/apktool.jar \
  .cache/decbench-repair/smali/Assemble.java "$fixture" "$prepared/classes.dex" 24
cmp "$prepared/classes.dex" "$fixture/classes.dex"
python3 quality/decbench run --binary path/to/diana \
  --suite dex-constructor-preallocation-order
```

The DecBench probe runs the Java explanation and the exact ordinary Diana
method body with the same helpers. The 0.4.7 baseline (`d60bc4cc`) publishes the
original member without diagnostics and moves both preparations into `new`.
All six recovered executions diverge from the scenario manifest. The six
reference executions match. The 0.4.8 candidate passes all six recovered
executions and preserves original identity and diagnostic coverage. The
ordinary Engine regression also checks that the preparation remains before
`new` and that the prior postallocation argument tree stays inside `new`.
[Semantic review](review.md)
