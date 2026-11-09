# Two-site loop semantic review

This LLM review compares authored Java/Kotlin, Smali, and Engine output using
the README model. The explicit loop graph remains a recovery boundary rather
than being counted as a recovered source loop. The dispatch-spelling candidate
does not change this fixture's text or issue list relative to `64aaa04`.

The initial route sets sum/index to zero. The head exits to the boxed current sum
for nonpositive limits or a completed loop. First-site publication saves limit,
sum, and index before load(index). State one restores those values and checks
failure before first conversion and record. A negative first result reaches the
same sum exit without second load, arithmetic, or increment. The first record
still occurs on that negative path, matching the authored body.

The second site saves first along with limit/sum/index, then prepares first + 1
and publishes two. State two restores that first value and checks the second
result before conversion. Only successful completion computes first + second,
adds it to saved sum, and increments saved index before returning to the head.
Both marker routes reach the original marker return before their tails.

For limit 2, contributions 2 and 8 give 10; for limit 3 the next contribution 14
gives 24. The third load returning -3 exits with the first iteration's sum 2,
after record(-3). The two argument-one calls refer to distinct site/iteration
positions, so a fault at the second versus third load has the separate effects
specified in README. Record or second-load failure does not update sum/index.
The callback's zero limit is replaced from I$0 on either resume. No contradiction
was found under the modeled field/helper assumptions.

Original references, shared tail copies, state/error paths and call origins are
retained. Future recovery needs both interior entries and the between-site break;
the current single-site plan is not used optimistically. Not run: helper effects,
scheduler behavior, or a final-plan behavioral interpreter.
