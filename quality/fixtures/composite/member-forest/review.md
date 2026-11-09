# Shared member-index review

LLM review compares the authored Java/Smali with ordinary dbg_shell output from
`02831ff` and the shared member-index replacement. Both inputs use the same prepared
DEX. P6 member-set comparison sorts only the enumeration; original operation and
edge IDs, child roles, entries, follows and completions are unchanged. P9 class
text is identical. Member enumeration is not the selected execution order.

In interleaved, the branch selects right for zero and left for either nonzero
scenario. The two portions of each arm are physically separated by the other
arm's portions. Output retains one branch with right's 2 + 5 and left's 1 + 3;
the shared result copy executes after the selected value. A common member-buffer
range can cover the entire private arm without changing its execution children.
The independent selected-plan model returns 7, 4 and 4 for 0, 1 and -1.

In nested, the negative guard returns -1 before every loop effect. The loop tests
the prior committed value against four. Cases zero and one name one static tick
body; case two adds two, while default adds one. Each selected result reaches
the original loop value through its own copy. A tick fault occurs before its
successful result commit, so the RuntimeException handler increments the prior
loop value. For the declared tick(1) fault scenario, T(0) commits one, T(1) throws,
the handler commits two, and T(2), T(3) finish at four. Start five exits immediately.
Typed protection, handler storage and shared switch keys retain their original
correspondence. The output contains no duplicate tick body or introduced route.

In locked, acquire precedes the loop. Null therefore fails before tick. With a
non-null acquired lock, each normal tick result commits before the next test.
Normal completion at three exits the synchronized scope once. A tick(1) fault
unwinds that scope after the reached calls and releases before the original
exception propagates. The source-only compiler release/catchall portions retain
origins without another executable release at the closing brace.

No contradiction was found on these reviewed paths. The ordinary child-leaf
traversal independently agrees with each delivered member set; pointer checks
confirm that nonempty member slices share the root buffer. Deliberately removing
one delivered leaf is rejected by the root-set assertion. These checks support
the representation invariant. They do not execute tick, arbitrary loops/switches
or JVM monitors. VM/device execution and complete recovery-design acceptance are
unrun. Earlier fixture output and reviews are reused only where before/after
facts and text agree on the six recorded fixture groups.
