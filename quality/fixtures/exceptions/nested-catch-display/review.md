# Nested and multi-clause catch display review

## Verdict and scope

The fixture was authored after the generalized display draft and before its first
inspection. Compared identical prepared bytes on
0d3da9a2595da9a2fa89a28f7a899a50f46d2a13 and the repair. On the baseline,
`layered` prints two chain lines, inline markers and handler labels even though
its printed nesting already states both original clause sets. `aliased` keeps a
chain beside a `catch (IllegalStateException | IllegalArgumentException)` union.
`catchBody` was already clean. The candidate prints no chain line or marker for
any subject. No contradiction was identified under the pre-inspection models; no
native, VM, device or Java-output execution occurred.

## Expectation revision

The first README draft expected `aliased` to keep its chain. It was revised before
this review was written: one printed union clause enumerates both original types
at that single handler position, and both clauses dispatch to the same body with
the same captured object, so clause order between them is unobservable. The
revision is recorded here rather than silently absorbed, and the rule still keeps
a chain whenever a printed handler position names fewer types than its original
clauses.

## layered

`step(0,mode)` prints before the inner clause. A matching A enters the inner
handler, so `recover(A,0,mode)` runs and is itself protected by the same-type outer
clause; its normal completion finishes once and returns the captured value. The
outer all-clause covers the normal body completion and both inner intervals, so a
nonmatching B/E from `step` reaches `finish` and is rethrown unchanged. A failure
in `finish` replaces the pending result or throwable after one call.

The decisive property is positional: the `catch (IllegalStateException)` that
contains the inner recovery text prints before the handler that covers it. Nested
`try` text therefore states the same priority the removed chain clause order
stated, including the equal type at both levels. Neither handler body gained or
lost a call, the row argument stays0 for the inner recovery and9 for the outer,
and all three original `finish` sites remain.

## catchBody

The second call textually sits in the first clause's body, inside its own
`try/catch(IllegalArgumentException)`. A failing `recover` therefore propagates
before the second call, and a failing second call returns-2 instead of reaching
the outer `IllegalStateException` clause, which does not protect its own catch
body. Both calls, the add and both returns keep their original order and operands.

## aliased

The printed union lists `IllegalStateException` then `IllegalArgumentException`,
matching the original adjacent clause order, and both reach one shared body using
the same captured object and the same row argument0. The body's single
`recover(same,0,mode)` and `-100-row` result are unchanged, so the union plus body
states everything the removed chain line stated. A later nonadjacent clause with
the same handler would produce two printed positions and keep the chain.

## Evidence

Nine ordinary-Engine checks in this iteration cover the plain typed shape, loop,
rethrow, the explicit shared-handler refusal, a two-handler chain, nested
typed+finally, two enclosing same-type handlers, a catch-body inner handler and
the union case. Baseline sensitivity is five passes and four failures, covering
every newly suppressed shape; the candidate passes all nine. Both isolated suites
pass173 checks. The116-fixture comparison keeps complete P2–P5 and
original-coordinate P6 facts, and the same APK keeps full coverage with only
`ChapterUtils.loadChapters` changing class text. These are product and manual
results, not execution.
