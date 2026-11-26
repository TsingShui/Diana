# Cross-owner helper move with two callers

This original Apache-2.0 challenge extends [swapped helper constants](../swapped-helper-constants/) by moving a method between owners that each retain identifiable methods. The two callers belong to the first owner in both releases. Java explains the arithmetic and calls in the assembled Smali.

## Independent expectations

- For signature `marker(I)I`, Lxx/e; and Lyy/g; call Integer.reverseBytes. Lxx/f; and Lyy/h; call Integer.bitCount. Both marker pairs have unchanged operations, independently identifying each owner.
- `Lxx/e;->m(I)I` moves to `Lyy/h;->n(I)I`. The old method XORs its argument with 17. The new one XORs with 19. Both then multiply by 5 and subtract 1 using 32-bit integer operations. At 16 the two returns are 4 and 14. The changed literal is at offset 0.
- `Lxx/e;->read(I)I` and `Lyy/g;->read(I)I` pass their argument to the corresponding helper, then apply Integer.highestOneBit. At 16 they return 4 and 8. `Lxx/e;->write(I)I` and `Lyy/g;->write(I)I` apply Integer.numberOfLeadingZeros to the helper return, yielding 29 and 28. Each caller preserves its own call order and argument use. Both distinct callers identify the same helper movement even though the owner's other methods stayed together.

## Preparation and reproduction

Use the [shared assembly procedure](../renamed-dispatch/#preparation-and-inspection) with `slug=cross-owner-helper-move`. Smali 3.0.9, OpenJDK 21.0.12.1 and API 24 assembled the before and after inputs.

```sh
diana diff quality/fixtures/comparison/cross-owner-helper-move/before/classes.dex \
  quality/fixtures/comparison/cross-owner-helper-move/after/classes.dex
cargo nextest run --locked -p diana --test diff_test -E 'test(two_callers_retain)'
```

The comparison implementation on base 7be3f1e paired the two markers and both callers, but left the old and new helper unmatched and attributed changes to both caller instructions. The [semantic review](review.md) records the actual data flow and outcomes.

On the revised matcher based on 7be3f1e the CLI regression above passed on the same prepared inputs. The check executes the comparison and inspects the original member pairs and changed DEX sites.
