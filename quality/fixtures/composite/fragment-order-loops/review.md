# Guarded switch-loop source/output review

LLM review compares the authored Java/Smali and ordinary dbg_shell output on the
retained `d5c665e` binary and the current refactor. The original P6 products, text
and reported issues agree. This is a structural replacement compatibility
challenge; the new benefit is retaining formerly absent constant origins.

The input start value is prepared before the zero-flag guard. Flag zero returns
7 without entering the loop. On the other path, the existing Phi/storage copy
initializes the loop local from that prepared value. The loop test compares it
with 4 before each selection. Starts 4 and 5 therefore return unchanged.

Keys 0 and 1 both select the one original small-update body. Its add-one result
commits and the original edge copy updates the loop local before repeating.
Key 2 instead commits add-two. The default body commits add-one. Each printed
break completes the switch and reaches the enclosing loop predicate; there is
no work after the switch to skip. Starting at -1 follows default, 0, 1, 2 and
returns 4; starts 0, 1, 2 and 3 follow the suffixes recorded in the README.
No original update becomes an initializer or update clause of a for loop.

Both case labels share one static body and one operation origin. Other updates,
copies and transfers remain distinct. The constants 4 and 7 now keep zero-width
operation/statement origins at their original portions. Their displayed uses
retain the existing rematerialization and do not add an execution owner.

No contradiction was found for the listed paths. The executed Rust checks cover
original ownership, source anchors and selected loop/switch structure. They do
not execute loop/graph plans, helper behavior or a VM. Full historical regression
remains deferred by the operator.
