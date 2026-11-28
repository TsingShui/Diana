# DEX annotation defaults review

The prepared file defines two original annotation interfaces. The system
carrier on `DexDefaults` has a single `value` which is another `DexDefaults`
annotation. The nested element names match the original declared zero-argument
methods. Its contents independently establish nine default bindings.

The original `count`, `name` and `enabled` entries use scalar integer, string
and Boolean encodings. The string includes a line feed and literal quotation
marks. The class entry identifies the array descriptor `[Ljava/lang/String;`.
The enum entry points to the `RUNTIME` field owned by `RetentionPolicy`, with
that same field type. The single `DexLabel` default binds both required child
elements. The annotation array has two child bindings in encoded order. The
integer array `scores` keeps its repeated value and signed middle value.
`empty` is present with zero elements. `required`, `rank` and `value` have no
binding on their respective declaring interfaces.

The frozen 0.4.22 publication retains the two annotation interfaces and their
members. It omits the nine default expressions. Exact recompilation succeeds,
and reflection reports all nine as absent. The three original no-default
controls agree. The raw DEX reader and authored Java reflection independently
agree on all twelve expected observations.

The repaired publication attaches each default to its named declaration.
Reflection confirms the class and enum descriptors, nested annotation
identity, signed values and array order on the exact published Java. The
three required elements retain their original declarations without defaults.

The verifier replaces the integer default for `count()` in a scratch copy
with a string reference. On the frozen baseline, class analysis and selection
of the malformed member produce no issue, and the eight valid defaults are
still absent. The repaired class and method publications identify the typed
mismatch on `count()I` with its original method ID 1. Selecting `labels()`
returns no issues and retains the exact annotation array. Reflection on both
the repaired class publication and the `labels()` shell confirms every valid
sibling default and the absence of a usable default on `count()`.
