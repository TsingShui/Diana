# Fixture: obf-flow-r8

R8-obfuscated enum state machine (switch, loop, try/finally, ternary). Built from the same Java reference as its source fixture with an
obfuscator, so the decompiled bodies can be compared against a known-good
reference.

- Provenance: authored for this repository. No third-party code or data; the
  obfuscated DEX is generated from our own source.
- Toolchain: R8, or ProGuard when the rules file starts with `# obfuscator:
  proguard`; `-dontshrink -dontoptimize` unless the rules say otherwise.
  `quality/build.sh` runs whichever tool the fixture's own rules file names.
- `mapping.txt` records original and obfuscated identities. Pair methods by
  original owner, name and full prototype; reused short names alone are ambiguous.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Current verdict — round 1, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `advance` | ✔ The START ternary, RUNNING nested If and empty else preserve the source selection. |
| `run` | ✔ The loop, continue/break and finally write-back agree with complex-flow.run. |
| `summarize` | ✔ The ternary expands into locals and If with equivalent selection. |

### Findings

- After R8 renaming, P7 selects when rather than switch, with the original numeric decision arms. Recorded forms: `when (v3) { 0 -> {…} 1 -> {…} else -> { } }`.
- MemberClasses remains; the field and its type share the short spelling a. Recorded forms: `@MemberClasses(value = {class a}) system`, `private a a;`.
- The when presentation avoids the early default label and gotos seen in the un-obfuscated view. Recorded forms: `default:`.

### Ranked fixes

Review when presentation for Java input; use the appropriate P7 decision form without changing original selections.

## Rounds

- R1 2026-11-08: After R8 renaming, P7 selects when rather than switch, with the original numeric decision arms. Recorded forms: `when (v3) { 0 -> {…} 1 -> {…} else -> { } }`.
