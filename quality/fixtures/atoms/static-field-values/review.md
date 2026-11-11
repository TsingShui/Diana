# Review: static-field-values

Compared the Java source and the same DEX on `b5eb94d` and the recovery-fixes
candidate. This is an LLM review of field preparation semantics, supported by
ordinary Engine integration assertions; no behavioral execution is claimed.

## Semantics

| Subject | Judgment and reasoning |
|---|---|
| Integer preparations | Agreement: CLOSE5, PREPARE_CAPTURE3, PREPARE_PREVIEW1, PROCESS_CAPTURE_IMAGE4 and PROCESS_PREVIEW2 retain their independently authored values in the correct declarations. The baseline incorrectly prints zero for every field. |
| String preparation | Agreement: LABEL retains `fixture`; the baseline incorrectly prints null. |
| Other behavior | The only differences are these six preparations. There is no new executable initializer or constructor call, so recovery does not add side effects. |
