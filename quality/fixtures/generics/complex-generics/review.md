# Fixture: complex-generics

## Current verdict — whole-operation follow-up

Read the changed repetition/traversal paths. Original calls, casts, selected
values and generic metadata remain; the original pure counter tail is supplied
at the same body end. No next call or cast moves across an earlier effect, and
zero-trip paths still skip body work.

The [pure-tail challenge](../../exceptions/pure-tail-finally/review.md) and
[carrier challenge](../../atoms/refined-carriers/review.md) record the owning
producer/consumer repairs and blocked drafts. This is bounded current source
reading; historical execution claims below were not rerun and do not certify
this output. No behavioral executor, device run or Java recompilation ran.


generic class, generic method with a bound, bridge method and generic call. The Java source is the reference; the DEX is generated from it with
`javac --release 8 -g` + `d8 --min-api 24` (see `quality/build.sh`) and committed
so a review needs no toolchain.

- Provenance: authored for this repository. No third-party code or data.
- Regenerate with `quality/build.sh`; never edit `classes.dex` by hand.

## Prior verdict — round 1, 2026-11-08

### Semantics

The verdict is written by reading the reference next to the output.

| method | semantics |
|---|---|
| `largest` | ✔ The erased Comparable comparison agrees; the generic bound remains only in Signature metadata. |
| `repeat` | ✔ The generic List<T> form is recovered. |
| `applyMapper` | ✔ Mapper<String> and its cast agree. |

### Findings

- Generic presentation is inconsistent: repeat recovers <T> List<T>, while largest retains an erased declaration plus Signature metadata. Recorded forms: `java.lang.Comparable largest(java.util.List)`, `@Signature(...) system`.
- Signature/MethodParameters annotations and their system visibility appear in the main text. Recorded forms: `@Signature`, `@MethodParameters`.
- The Identity.apply bridge was not separately reviewed; its inner class is rendered separately.

### Ranked fixes

Improve generic signatures from available metadata, annotation presentation and parameter names. Recorded forms: `@Signature`.

## Rounds

- R1 2026-11-08: Generic presentation is inconsistent: repeat recovers <T> List<T>, while largest retains an erased declaration plus Signature metadata. Recorded forms: `java.lang.Comparable largest(java.util.List)`, `@Signature(...) system`.
