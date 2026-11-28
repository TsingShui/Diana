# Semantic review

The two origin classes and their outside counterparts share simple names
and declare identical field names and types. The raw DEX binds each
`sput` to an outside class. Both original observers read the unchanged
local default and the new outside value.

In the prefix case Diana 0.4.13 moves the store into the wrong declaration.
In the delayed case the initial call runs, and the following store loses
its qualifier. The original begin callback observes the outside default
before the store. The baseline keeps that callback result and changes both
values observed after initialization.

Diana 0.4.14 prints the external member with its type qualification and
retains the delayed store after the begin callback. The raw member checks
and the independent Java caller confirm the two correct pairs of values.
