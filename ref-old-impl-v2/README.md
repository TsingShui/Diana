# Old Diana staged pipeline, second reference copy

Historical copy of the P2–P9 staged recovery chain as it stood when the backend
became the ordinary `Engine::analyze` entry.

- Source commit: `0dd038b`.
- Contents: `p2`, `p3`, `p4`, `p5`, `p6`, `p7`, `p8`, `p9`, `recovery` and the
  P2–P7 chain they form, including the unified statement roots and the single
  statement placement record.

This tree is reference material only. It is not part of the workspace build, is
not compiled, tested or linted, and must not be imported by active code. The live
crate still carries the same modules until the staged entry is retired; keep both
copies consistent only while that is true.
