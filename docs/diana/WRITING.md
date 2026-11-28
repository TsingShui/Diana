# Writing Diana documentation

Write repository documentation in English. Keep terminology, commands,
identifiers, units and examples consistent with the owning code and package
documentation. Preserve the meaning and strength of technical requirements
when editing for style.

- Use direct language for obligations and explain recommendations where the
  reason affects a decision. Use imperative steps for procedures.
- Describe the contract near its owner. Put current usage and status in
  READMEs, contributor rules in `AGENTS.md` or the relevant guide, and
  implementation invariants near the code that enforces them. Link to the
  authoritative description when another document needs it.
- Describe implemented behavior, design intent and observed results separately.
  State the input, revision, command and scope when they matter to an evidence
  claim. Identify uncertainty and unrun checks. Use tracked paths and pinned
  revisions for shared evidence.
- Give examples with resolvable links and realistic inputs. Distinguish a
  written semantic review from an executed regression check. Keep independent
  expectations tied to the original input and product contract.
