# Review: recovery-warning-members

This LLM review compares the authored Smali and Java explanations with the same
prepared DEX's baseline and candidate class/selected-method publications. It
judges diagnostic attribution and readable sibling behavior; it does not claim
runtime equivalence or ART acceptance of the deliberately malformed bodies.

| Subject | Judgment and reasoning |
|---|---|
| `RecoveryWarningMembers.broken` | The DEX returns an uninitialized allocation. Recovery remains unavailable. The baseline's generated executable throw could be mistaken for original code. The candidate instead marks unavailability with Diana warning comments, retaining the concrete missing-constructor reason and issue URL. It fabricates no replacement execution. |
| `BrokenConstructor.<init>` | The DEX calls this object's super constructor, then publishes another uninitialized allocation. Recovery remains unavailable. The candidate keeps the constructor declaration and warning; it is not removed as an empty default constructor. The structured identity remains `<init>`, not the source constructor name. The warning does not claim to reproduce the unavailable prefix or its publication. |
| `BrokenInitializer.<clinit>` | The DEX publishes an uninitialized allocation from the static initializer. Recovery remains unavailable. The candidate retains a visible static block with the warning instead of suppressing it as empty. Its issue retains `<clinit>`. The comment does not claim that the original static initialization did no work. |
| `RecoveryWarningMembers.originalThrow` | Agreement on the recovered behavior: allocate the original UnsupportedOperationException, initialize it with the exact authored message, then throw it. That message deliberately resembles the old Diana diagnostic. The output remains an executable original throw with no recovery issue; no text-based substitution is applied to it. |
| Three `sibling` methods | Agreement: the original constants are returned as 7, 9 and 11. These bodies are unchanged between baseline and candidate. Their selected-method requests succeed independently of the failed members. |
| Class versus selected-member delivery | The unavailable scope and original identity stay the same in both presentations. Available siblings remain readable. Text/JSON parity, issues, exit codes and stderr were inspected separately from the semantic judgment. |

The challenge preserves the original missing-constructor trigger while adding
member-kind handling, declaration retention and the original-throw ambiguity.
It therefore exercises the shared diagnostic producer and printer across all
three body-owning declaration kinds without weakening the recovery failure.
