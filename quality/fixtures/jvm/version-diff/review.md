# Original bytecode and comparison review

The pre-feature command reaches the DEX selector and cannot inspect the JVM
method definitions. The comparison checks the following original instructions
and declarations with the same index, pairing and alignment used for DEX.

- Both versions of `same()` load the original UTF-16 sequence for
  `shared\0🌱`. The narrowed pool index selects `ldc` in the first version
  and the padded pool selects `ldc_w` in the second. The later return changes
  byte position, yet the method's normalized structure and value agree.
- `message()` changes its returned text while the coordinate of the original
  class also changes. The report pairs the original linkage name and shows the
  two string values at byte zero. The nested version produces physical method
  positions that retrieve precisely those two values.
- `branch(I)I` retains the `Math.abs` call for a nonzero parameter. Its zero
  test changes destination from byte 9 to byte 11, changing the zero path's
  return from -1 to 0. `route(I)I` keeps the three keyed destinations and
  changes the default from byte 40 to byte 42, changing only the default return
  from 2 to 3. The comparison reports each modified transfer at byte 1.
- `caught()I` executes `athrow` on a null operand at byte 1. The old table
  applies `NullPointerException` before `Throwable` at this site, producing
  1. The new table applies `Throwable` first, producing 2. The comparison
  retains the full original order even when a later clause is shadowed.
- `generic()` is native. Its erased descriptor remains the same. Reflection
  verifies the declared return type arguments and throws type change, and the
  report locates those changes in the declaration.
- `argument()` changes one symbolic bootstrap argument. Reordering the
  bootstrap table alone keeps the original handle and argument unchanged.
  In `sharing()`, the first version loads the same dynamic constant twice.
  the second version has two structurally identical but physically separate
  entries. The comparison records this distinction at the second load.
  `Verify` checks the separately assembled bootstrap's result and confirms
  one invocation for the shared case versus two for the distinct entries.
- The damaged class envelope and malformed suffix leave the intact class and
  preceding instruction available. The malformed method is reported unknown
  with its changed prefix. The duplicate archives leave all physical candidates
  visible without claiming pairs.

The OpenJDK fixture execution and independent `javap` inspection were run.
The damaged and duplicate inputs were inspected through the CLI and were
excluded from the execution check. Class-wide bootstrap sharing across methods
and behavior of unknown external bootstrap implementations remain outside the
comparison's method-local evidence.
