# Archived Diana implementations

This directory keeps historical implementations as reference material.

- `v1/`: pre-refactor P2–P8 stages, declaration context and Engine helpers.
- `v2/`: staged P2–P9 recovery chain kept when the backend became the
  ordinary `Engine::analyze` entry.

Neither tree is part of the workspace build. They are not compiled, tested
or linted, and active code must not import them.
