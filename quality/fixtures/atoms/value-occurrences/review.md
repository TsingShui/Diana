# Value-occurrence semantic review

Baseline: f83f2fa. Candidate: the accompanying uncommitted
work/value-aware-recovery changes based on f83f2fa. Both consume the same
prepared DEX through ordinary Engine requests.

## Path-by-path judgment

- chain: the two identity Moves do not add effects. The selected return evaluates
  addition before multiplication, exactly once, with wrapping int arithmetic.
  The recorded overflow case still returns the minimum int. No storage read
  relies on a removed definition.
- effects: tick remains one materialized call before pair. Its argument is x,
  pair receives the saved x+7 and literal 2. The former self-move carries tick's
  evaluation instead of silently discarding it. visits changes once.
- repeated: the inner tick completes before becoming the outer tick's argument.
  With x=3, the arguments are 3 then 10, the result is 17 and visits changes twice.
  Composition does not duplicate either helper.
- shared: the saved value is still assigned once and read twice. The output does
  not duplicate tick in pair's arguments; its declaration still covers both reads.
- bytes: allocation and the exact primitive payload are adjacent, with no
  intervening publication. One fresh [1,2,3] object is returned. Other values that
  previously shared its execution storage do not become its array identity.
- checked: identity still precedes the String check and length. A standard
  String returns its length; null passes the cast and fails on length; another
  object fails the cast before length. This is source reading, not device or
  executed object/exception checking.
- identity, pair and tick retain their authored helper behavior. In particular,
  tick increments visits before its wrapping addition. No external declaration
  was introduced into engine input.

## Supporting checks and limitations

The integration test executes the final linear integer plan for chain, effects,
repeated and shared using explicit helper models, and checks original call and
nonliteral-operation anchors. The checker has an extra-helper-effect negative
case. It does not execute arrays, casts, catches or a JVM.

The grouped constructor-argument compatibility case confirms that private
prepared values remain hidden after centralized materialization selection.
The [effect challenge](../value-occurrence-effects/review.md) adds shared
arithmetic, two fault positions, protected state and interleaved array publication.
Builds and written agreement do not prove arbitrary-program equivalence.
