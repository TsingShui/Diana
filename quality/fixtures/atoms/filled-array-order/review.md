# Semantic review

FairEmail's analyzed `TupleFolderSort.getSortTitle` evaluates a virtual call and
an account-name field read before allocating the returned array. The 0.4.10
publication binds both completed values to locals before `new String[]`. An
allocation fault now occurs after those reads. In the nested `AI.completeChat`
case, the conversion and truncation likewise precede the `String[]` supplied
to the original explicit initializer.

In `filled`, successful execution records both producers and then the array
allocation before returning their values. Either producer fault prevents the
allocation. The modeled allocation fault leaves both producer events in the
trace and escapes with its original identity. The 0.4.9 initializer tries the
allocation first in all four cases. `stored` already allocates before the two
producers. Its natural initializer expresses the original DEX order, and both
builds pass its four scenarios.

`protectedFilled` records the first producer and copies its reference. The
candidate evaluates the second producer before allocation and preserves the
same first reference at slots 0 and 2. Either producer fault enters the typed
handler with stage 80 and its original exception. The allocation error occurs
after both producers and passes through the typed catch. The older output
allocated before the second producer, which changes the normal, second fault
and allocation fault traces.

`staged` leaves the catch state before each operation. A fault in the first
producer reports stage 10 with two null references. Later faults report stage
20 with the first reference, or stage 30 or 40 with both references. The two
publications keep these execution paths and return the original values and
aliases on success. The original DEX witness and the independently executed
Java explanation agree on all 17 scenarios.
