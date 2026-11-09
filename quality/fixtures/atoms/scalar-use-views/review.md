# Scalar-use views review

## Current verdict

C1 reproduces on df10b744985e3407f902442b8252fab100700aec: carrier's char
probe is printed with an int carrier and no use disclosure; replace's char
constants print as bare decimal arguments. The candidate consumes the existing
P4/P7 views, spelling carriers with `via` and exact character literals in quotes.
It does not change callee identity, storage or any P2–P8 product.

Array and boundary calls already had signature disclosure on the baseline;
those are view-clarity/compatibility checks, not additional behavioral defects.
The declared char parameter is a compatibility case too. Its repeated char view
is explicit but redundant with the header; no global local-retyping was added.

### Semantics

These are bounded LLM source/Smali/text judgments under the README's models,
not executed behavior PASS results.

| Method | Judgment and reasoning |
|---|---|
| `carrier(String)I` | ~ charAt runs once before char probe then int probe. Both argument occurrences still read the same word. Only the char use gains its supplied view. For A, helper arithmetic gives1065+2065=3130, not two int-probe results. An initial charAt failure skips both calls and addition. |
| `array([C)String` | ~ Constructor precedes the load; char append precedes int append and toString. Under the stated library model A gives A65, not 6565. Null/bounds failure skips both appends. The original char/int signature qualifiers remain. |
| `literal(String)String` | ~ One original replace(CC) receives slash47 and dash45 in order; quoted characters preserve those exact units. Null still fails at that original call. |
| `escaped(String)String` | ~ Quote39 prints with an escaped quote; surrogate0xd800 remains an escaped single UTF-16 unit, not a replacement scalar or pair. The one replace call preserves its original order and binding. |
| `boundaries()I` | ~ Char65535, signed byte-128 and signed short-32768 retain their numeric values under the supplied lossless views. Boolean true/false retain P7's existing selection. Probe/add order yields50640 under the helper bodies. |
| `parameter(C)I` | ~ Original char header and char binding remain; the explicit view evaluates the parameter once without a cast/check operation. |
| `probe(C)I` | ~ Returns1000 plus the incoming unsigned UTF-16 unit as an int; arithmetic read is not rewritten as a char operation. |
| `probe(I)I` | ~ Returns2000 plus the original int with int wrapping. |
| `probe(B)I` | ~ Returns3000 plus the original signed byte value, without unsigned masking. |
| `probe(S)I` | ~ Returns4000 plus the original signed short value. |
| `probe(Z)I` | ~ Under canonical boolean inputs, zero returns5000 and one returns5001. The existing word comparison is unchanged, not a Java-compilation claim. |

The same prepared input and final test source ran against the baseline and
candidate. The original six checks were one pass/five failures before repair;
all six pass after repair. The full 13-check set also covers the protected
[complexity challenge](../../exceptions/scalar-views-protected-joins/review.md)
and separate retry/cleanup preservation inputs. Exact P2–P8 comparison and origin
checks support this reading but do not execute these scenarios.

## Rounds

- Range-proved scalar disclosure: repaired the common read/literal consumer;
  no StringBuilder-name special case or external declarations.
