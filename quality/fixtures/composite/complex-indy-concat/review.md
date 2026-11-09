# Fixture: complex-indy-concat

string concatenation through StringConcatFactory (invoke-custom). The Java source is the reference; the DEX is generated from it
with the fixture's `javac.args`/`d8.args` or the shared defaults (see
`quality/build.sh`) and committed so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/ComplexIndyConcat;`

## Current verdict — inferred concat presentation and scalar views

The scalar-use change was compared with df10b74 on the same DEX. Only many's
int-stored char gains an explicit supplied view inside its existing P7-selected
char conversion. That annotation is redundant but value-preserving: no second
producer, runtime cast instruction or different String conversion is introduced.
Its delimiter/capture order and inferred-bootstrap marker remain. P2–P8 products
are exact; simple/insideLoop text is unchanged, so their bounded judgments below
are reused. This is LLM reading, not executed linkage or behavior verification.

### Semantics

| method | semantics |
|---|---|
| `simple` | ~ Under the standard bootstrap contract, p0, colon and p1 occupy their original positions and give the same String content. |
| `many` | ~ String, int, long and char views retain the same argument sequence; preparation and the char conversion remain explicit. |
| `insideLoop` | ~ Captured result/bracket/element/end values concatenate before the original index/result updates. |

The current reading compares baseline `8785d2f` and candidate outputs on the same
DEX. P7 retains the original Custom invocation and complete bootstrap definition;
the expression marks the StringConcatFactory interpretation as inferred. Runtime
linkage/library replacement and allocation behavior are not proved.
Simple resolves the colon at its admitted original operand position before the
first join, then uses the count. Many preserves the original @, %, slash and #
preparation order and all captured values; its int-stored char is explicitly
viewed as char, so A stays A rather than 65. Long uses the delivered integral long
view. InsideLoop reads the element before resolving its delimiters, computes the
new String, then keeps index/result updates in their original order. The closing
delimiter remains a statement because it follows the first printed join.
Null input still fails at the original initial length read; an empty array returns
the original empty result. No new helper invocation or loop syntax is inferred.

### Findings

- The call now has a readable `+` presentation, with inference and original
  call-site disclosure. Complete bootstrap metadata remains available.
- Bootstrap method, prototype and argument count remain available; no replacement facts are invented.

### Remaining boundary

Arbitrary bootstrap behavior, unsupported object/static-constant conversions and
missing argument views retain their original explicit invocation. The inference
does not certify runtime linkage or allocation behavior.

## Rounds

- Scalar-use disclosure: reread many's char operand; its existing conversion/inference, captured values and effect order remain.
- R1 2026-11-08: invoke-custom prints explicit call-site metadata for StringConcatFactory.makeConcat followed by the original invocation and arguments.
- R2 2026-11-09: Recovered the three original concat bodies and reviewed captures, conversions and update order; the new complexity challenge is [concat-capture-recipes](../concat-capture-recipes/review.md).
