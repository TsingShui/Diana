# Fixture: class-bridge

bounded generics, a real bridge method and wildcard signatures. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.
- Selector: `Lfixture/ClassBridge;`

## Current verdict — round 1, 2026-11-08

### Semantics

| method | semantics |
|---|---|
| `countAbove` | ✔ |
| `copyInto` | ✔ List<? super T>.addAll agrees. |

### Findings

- The bridge method retains its identity but exposes `dex_flags<bridge synthetic>` in its declaration. Recorded forms: `public dex_flags<bridge synthetic> int compare(Object, Object)`.
- Generic bounds survive only as fragmented @Signature text rather than a Box<T extends Comparable<T>> declaration. Recorded forms: `implements java.util.Comparator`, `<T::`, `Ljava/lang/Comparable<`.
- items uses a raw List declaration, while copyInto preserves its generic operation; presentation is inconsistent. Recorded forms: `private final java.util.List items`.

### Ranked fixes

Improve generic declarations/bounds from available Signature facts and the presentation of bridge/synthetic flags. Recorded forms: `@Signature`, `dex_flags<bridge synthetic>`.

## Rounds

- R1 2026-11-08: The bridge method retains its identity but exposes `dex_flags<bridge synthetic>` in its declaration. Recorded forms: `public dex_flags<bridge synthetic> int compare(Object, Object)`.
