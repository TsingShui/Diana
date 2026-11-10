# Vendored dexdec

This directory is a fork of the Rust decompiler core used by
[rust_ASC](https://github.com/MG1937/rust_ASC), taken from its upstream:

- Upstream: https://github.com/asLody/dexdec
- Revision: `08ed478781ddddb16381c31b9cf6d8946fdf2919` (`main`, 2026-11-05)
- License: Apache-2.0, see `LICENSE`

The copy is the complete upstream workspace: `dexdec` (the decompiler core) and
`rusty-dex` (its DEX reader), including upstream tests. Nothing was removed or
rewritten for the fork, so `git diff` against the recorded revision states every
local change.

## Why it is a separate workspace

`vendor/dexdec` keeps its own `[workspace]` table, and the repository root
excludes it. A path dependency on `vendor/dexdec/dexdec` therefore builds the
vendored core without making its members part of this workspace, its lint
settings or its default build set. `cargo check --manifest-path
vendor/dexdec/Cargo.toml -p dexdec` verifies the fork on its own.

## Change policy

- Record the upstream revision in this file whenever the fork moves.
- Keep local changes small, documented and separable from upstream code, so the
  next sync can be reviewed as a diff rather than a merge.
- Upstream is vendored, not published: a defect found here is fixed here.

## Cost

216,641 physical Rust lines across 308 files (16 MB of source). The core parses
a whole DEX image once per `Decompiler` and caches method IRs, hierarchy and ABI
facts per class-graph revision. Construct one `Decompiler` per DEX image and keep
it alive across requests; do not rebuild it per request.
