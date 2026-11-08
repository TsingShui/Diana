# Fixture: obf-dp-r8

R8-obfuscated edit distance, LIS and knapsack. Built from the same Java reference as its source fixture with an
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
| `editDistance` | ✔ The body matches algo-dp.editDistance; two-dimensional creation still uses Array.newInstance. |
| `longestIncreasing` | ✔ The body matches its source fixture. |
| `knapsack` | ✔ The body matches its source fixture. |

### Findings

- The recorded body comparison found no additional structure loss from renaming.
- Methods become a/b/c; Array.newInstance and array reads lacking casts remain visible.
- Use mapping.txt and original parameter types to pair obfuscated members with their source identities.

### Ranked fixes

As for algo-dp, improve two-dimensional declarations/reads and loop presentation.

## Rounds

- R1 2026-11-08: The recorded body comparison found no additional structure loss from renaming.
