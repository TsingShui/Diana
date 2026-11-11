# Fixture: direct-initialization-effects

This independently authored complexity challenge extends
[direct-initialization](../../classes/direct-initialization/README.md). It keeps
the different allocation/constructor owners while adding an alias, an effectful
conditional path, a protected allocation/initialization sequence and publication.
No application or identifying corpus data is included. Smali is authoritative;
the Java native declaration explains a body that Java cannot express directly.
Assembly success does not establish ART verification.

Subject: `Lfixture/DirectInitializationEffects;->build(I)Lfixture/Child;`.
Pre-fix revision: `b5eb94d`. Candidate: recovery-fixes branch.

## Independent scenarios

Each scenario starts with trace 0, last null and the stated fault value.
`event(n)` appends n to trace before checking for its configured exception.
Only Parent's constructor runs; Child's constructor would overwrite value with
99 and must never be substituted.

| argument | fault | Expected behavior |
|---|---|---|
| 7 | 0 | Allocate once; event1, Parent/event2, event4; publish and return the same Child alias with value7; trace124. |
| -7 | 0 | Allocate once; event1, event3, Parent/event2, event4; publish/return the same alias with value-7; trace1324. |
| 7 | 1 | event1 throws before initialization; catch runs event5 and rethrows the captured error; trace15, last null. |
| -7 | 3 | event1 then event3 throws; no constructor or event4/publication; event5 and rethrow; trace135. |
| 7 | 2 | Parent records event2 then throws before its value store; event5 and rethrow; trace125, last null. |
| 7 | 4 | Initialization succeeds; event4 throws before publication; event5 and rethrow; trace1245, last null. |

A VM allocation/linkage Error bypasses the RuntimeException handler and performs
no later work. Constructor resolution is retained as the original DEX call;
this fixture does not assert that a runtime accepts every optimized DEX form.
The handler's event5 is outside its own protected range. If that call independently
fails, its error escapes rather than recursively entering the same handler.

## Preparation

Use the parent's assembly command with the slug `direct-initialization-effects`
and source directory `quality/fixtures/exceptions/direct-initialization-effects`.
The recorded environment is Smali 2.5.2 and OpenJDK 21.0.12.1. The prepared DEX is
committed beside these files; exact assembler versions are not admission gates.
