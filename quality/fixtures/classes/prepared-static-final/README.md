# Preparation-only static finals

This independently authored DEX challenge extends
[the JVM preparation-only case](jvm/) and the
[runtime static final fixture](../runtime-static-final/). It combines a child
with omitted static values and no class initializer, an observable superclass
initializer, and a control with an actual encoded default-valued integer.

## Original input and independent expectations

`Lfixture/PreparedDefaults;` extends `Lfixture/PreparationParent;` and declares
four static final fields in static-field order. The encoded array contains
`A_ENCODED_ZERO:I` as integer zero and `B_ENCODED_SENTINEL:I` as integer nine.
The final two fields, `C_PREPARED_ZERO:I` and
`D_PREPARED_NULL:Ljava/lang/String;`, have no encoded values. Both retain their
descriptor defaults. The sentinel ensures assembly retains the preceding
encoded zero.

`Lfixture/ReadPreparedDefaults;->preparedZero()I` uses `sget` and
`Lfixture/ReadPreparedDefaults;->preparedNull()Ljava/lang/String;` uses
`sget-object`. A first active read of either child field initializes its
superclass before returning the default. The parent calls `start()` and
`finish()` in that order. `PreparationEvents.java` supplies the independently
authored model for these native hooks. A normal `finish()` records its event
and returns. A failing `finish()` records a fault and throws the exact modeled
exception. The first read then reports that exception as the initialization
cause. A subsequent read encounters the erroneous class without rerunning
either hook. Every scenario runs in a fresh process.

The independent `encodedZero()I` control contains `const/4 0` and returns with
no class access. Both before and after a failing initialization, that method
returns zero with no hook event. [The scenarios](scenarios.json) give exact
expected traces for both first-read orders and both normal and fault paths.

The Java explanation uses a nonconstant expression for the integer default
because a Java source final requires initialization. It models the same
values and order in these scenarios. The Smali source and assembled DEX
establish the absent preparation and the absence of a child initializer.

## Preparation and verification

The assembled input uses Apktool 2.11.1's embedded Smali assembler through the
local `Assemble.java` adapter at API 24. The adapter accepts the Smali
directory, output path, and API. The observed invocation was:

```sh
fixture=quality/fixtures/classes/prepared-static-final
work=.cache/fixtures/prepared-static-final
mkdir -p "$work"
java -cp "$APKTOOL_JAR" "$SMALI_ADAPTER" "$fixture" "$work/classes.dex" 24
cp "$work/classes.dex" "$fixture/classes.dex"
```

The probe reads field order and the encoded array from the original bytes.
It also checks the exact DEX read operands, control return, superclass and
ordered hook invocations. It executes the authored Java model and the
recovered class publications separately with OpenJDK 21.0.12.1. The probe
compares their traces with the scenarios. Execution covers the Java model and
recovered source for this fixture. The DEX check inspects original
instructions and preparation metadata.

```sh
python3 quality/decbench run --binary /path/to/diana \
  --suite dex-prepared-static-finals
cargo nextest run --locked -p diana-decomp --test static_field_values_test
```

The same DEX reproduced three trace failures on the frozen Diana 0.4.16
release. An integer read returned zero before the initializer on both the
normal and fault paths. A subsequent integer read after a failed null read
also returned zero. The working-tree candidate produced the expected trace
in all five scenarios. See the [semantic review](review.md) for the
first differing effects and their provenance.
