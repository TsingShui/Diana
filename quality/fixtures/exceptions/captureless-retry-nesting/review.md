# Captureless retry nesting review

## Verdict

Compared identical prepared bytes on 73fc8793a91d766d021d117aa83ddfd644f7ac82
and the P6 exception-rejoin repair. Read the Java explanation, actual Smali,
pre-inspection native scenarios and complete before/after output. The candidate
recovers both nested loops and the retreating-latch loop. The genuine pre-loop
arrival remains explicit. No contradiction was identified under the closed
README model; native/runtime behavior outside that model remains unknown.
This is LLM path reasoning supported by product checks, not executed behavior.

## Paths

Nested initializes row0/total0, tests rows, initializes column0 at each row and
tests columns. An entered pair calls attempt before advance. Success adds exactly
its result; IllegalStateException skips that absent result and reaches the same
original advance entry. The exception-only sum receive stays in
on_exception_entry, so normal success does not restore an older sum. Advance is
outside the attempt's interval: even the selected catch type thrown there escapes
without another increment, call or outer-row advance. Other attempt failures
escape before advance. Normal advance then increments column; inner exhaustion
increments row. This retains totals8/7/5 and every event prefix in the table,
including empty/negative bounds. No capture was introduced.

Terminal preserves the old index before incrementing it, then increments before
attempt. Both normal completion and the ignored exception reach the original
goto back to the predicate. The handler does not repeat the increment or restart
the failed iteration. Results3/2 and the nonpositive-limit no-call path remain.
The original PC6 increment still precedes PC8's invocation.

Foreign performs the prefix attempt before the first header. Its ignored failure
enters the same increment used by later body failures, advancing index0 to1
before the first test. Ordinary prefix success does not advance there. Thus the
normal case totals3, the selected prefix fault totals2, and a zero limit still
performs the prefix event. A while whose only entry were its header would hide
that exceptional bypass; the candidate keeps the graph and one shared entry.

## Evidence and limits

Three ordinary-Engine checks cover loop ownership, original latch identity,
exception-only receives, propagation-only advance, no fabricated captures and
the foreign refusal. The nine-check iteration set gives two passing refusals and
seven failed recoveries on the fixed baseline, then nine passes on the candidate.
The full isolated suite passes 155 checks in both configurations. Complete P2–P5
and original-coordinate P6 facts are unchanged across the 111-fixture comparison.
These checks do not run the native callbacks, a DEX VM or decompiled Java.
