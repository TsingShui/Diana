# Fixture: kotlin-null-safety

Kotlin null-safety forms and the DEX they produce: `?.`, `?:`, `!!`, and a chain of
safe calls. The LLM review below compares semantic behavior against the Kotlin source.

- Provenance: authored for this repository. No third-party code or data.
- Prepared with `kotlinc -jvm-target 1.8` + `d8 --min-api 24 --lib kotlin-stdlib.jar`
  (see `quality/build.sh` output for the versions in use).
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand. `kotlinc`
  output is not byte-reproducible (rebuilding changes the DEX bytes while the
  decompiled result stays identical), so a rebuild-only change to this file needs
  no commit.

## Current verdict — round 1, 2026-11-08

### Semantics

| method | semantics | shape | notation |
|---|---|---|---|
| `lengthOrZero` | ✔ `p0 == null` → `0`, else `p0.length()` | ~ `?:` becomes an if/assign pair | ✔ |
| `requireLength` | ✔ `Intrinsics.checkNotNull(p0); return p0.length();` — exactly what `!!` compiles to | ~ `!!` not recognised as such | ✔ |
| `plainParameter` | ✔ `return (p0 + 1);` | ✔ | ✔ |
| `chained` | ✔ null checks, `StringsKt.trim`, `Locale.ROOT`, `checkNotNullExpressionValue` all in order | ✘ two `?.` became `goto L5/L6` with a shared `null` exit | ✘ labels and `goto` |

The class keeps `@Metadata(...)`, printed with a trailing `runtime` token and its
full `d1` string; `kotlin.Metadata`, `kotlin.jvm.internal.Intrinsics` and
`kotlin.text.StringsKt` are imported and called with their original identities.

### Findings

- Kotlin's own idioms are not recovered: `?.`/`?:`/`!!` stay as explicit null
  branches and `Intrinsics` calls. That is faithful but not readable as Kotlin.
- A chain of safe calls (`chained`) is the worst case: two `goto`s and a shared
  `L5: v6 = null; L6: return v6;` exit instead of `text?.trim()?.lowercase()`.
- No `Intrinsics.checkNotNullParameter` appears: `plainParameter` is a `final`
  class method, so kotlinc emitted none. The fixture records that as a fact instead
  of assuming an entry check.
- `@Metadata` renders as `@Metadata(d1 = "...", d2 = {...}, ...) runtime`: the
  annotation is preserved (good) but the visibility word leaks into the text.

### Ranked fixes

Recover `?.`/`?:`/`!!` shapes; stop printing the annotation visibility word; keep
`Intrinsics.checkNotNullExpressionValue` attached to the read it guards rather than
as a separate statement.

## Rounds

- R1 2026-11-08: semantics correct in 4/4, safe-call chains fall back to `goto` labels.
