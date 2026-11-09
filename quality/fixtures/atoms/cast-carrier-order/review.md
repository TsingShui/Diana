# Semantic review of carrier operand order

LLM review compares authored Java/Smali with the delivered P9 path on `dd3a114`
and the containing shared-walk refactor. The independent selected-plan executor
also checks the recorded results, exceptions and helper arguments. Its supported
subset is explicit; this is not a VM/device or Java compilation result.

Right now prints the original check in the comparison's right operand. The
preceding left parameter read has no effect, so cast failure still precedes
reference comparison. Successful casts preserve identity, including null.
The original operation, statement and exception-site anchors each occur once.
Left retains the existing first-operand check behavior.

Argument now prints the check after integer zero and before sink. Evaluating zero
has no effect; incompatible values therefore prevent sink, and sink faults still
occur after the check and exact S(0,value) event. The baseline kept a separate
check. This is an additional recovery effect of the shared first-read query.

Guarded retains its standalone cast before the flag. In particular, an
incompatible right throws even on the path returning 99. AfterEffect similarly
retains the cast before make: cast failure skips M, successful/null casts preserve
the returned input reference, and make's own fault escapes after M.

ArrayLength retains its earlier inline cast. Null passes the cast and then fails
at length; an incompatible object fails the cast first. The input lengths 3 and
5 remain exact. No cast or call is duplicated, and no equals call replaces DEX
reference identity. No contradiction was found on the reviewed parent scenarios.
