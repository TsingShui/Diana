# Semantic review of cast preparation ownership

This LLM review compares authored Java/Smali with the output on `33b8abd` and the
containing readonly-check refactor. Independent selected-product execution also
checks the explicit fixture model. It is not VM/device execution and consumes no
external declarations.

## Discovery

Baseline ordered prints flag OR ((int[]) receiver).equals("x"). Its zero-flag
path performs the receiver's cast before the string operand. The original DEX
resolves x before that cast. On an incompatible O, baseline execution produces
CCE with no S event; the independent input expectation is S followed by CCE.
If resolution itself faults, the input expects that fault before the cast.
The baseline sensitivity case fails this exact missing S prefix.

The cause is the condition argument cover treating every check-cast as transparent
because it writes no register. It moved the string producer across a required
throwing operation. This is an operation-ownership error, not a missing comparison
or method-name template.

## Candidate paths

Ordered now retains String v2 = "x" before the call whose receiver owns the
required cast. Its explicit first flag path reaches the shared return 1 without
preparation. The zero-flag path runs S, then cast, then attempted E. O therefore
fails after S; a resolution fault prevents cast/E even for O. Null passes the
cast and faults on the attempted equals call after S. The string value's extra
statement and explicit transfers preserve order and one shared result arm.

CastFirst has the opposite source order. Its cast and string are compatible
with receiver/argument order, so the candidate retains a compact OR condition.
O fails before S; valid/null receivers reach S first, then the attempted call.
Nonzero flag still skips all work. Fixing ordered does not forbid valid casts in
an operand forest.

Produced keeps produce before check before consume. The producer's result store
prints inside the cast's operand, retaining its original materialization and
normal-result commit. A producer failure skips store/check/consume; O fails after
P and before C; a consumer failure follows P,C. Native helpers execute once.
The temporary remains because observer retirement is not part of this slice.

Two nextest cases cover 29 modeled paths: sixteen normal exits and thirteen
escaping faults, with exact effect/argument traces. Each check's original
operation, statement and site have one anchor. The earlier integer/reference
matrices also pass through the same test-only executor. There is no contradiction
on these reviewed paths. Historical fixture regression and VM behavior remain
unrun during the structural rewrite.
