# Swapped constants in renamed helpers

This original Apache-2.0 pair extends [renamed dispatch](../renamed-dispatch/) with a conflict between method-body equality and caller context. Two routes retain their distinctive library calls while their helpers exchange literal values. The new owner and helper names are independently selected. The Java classes explain the Smali sources.

## Independent expectations

- `Lx/a;->route(I)I` and `Ly/s;->route(I)I` call Integer.reverseBytes first. They pass its return value to their own XOR helper. At 1, the old result is 0x01000007 and the new result is 0x0100000f.
- `Lx/b;->route(I)I` and `Ly/t;->route(I)I` call Integer.lowestOneBit first. At 16 they pass 16 to their helpers and return 31 and 23 respectively.
- The first helper moves from `Lx/a;->a(I)I` to `Ly/s;->b(I)I` and changes the literal at code-unit offset 0 from 7 to 15. The other helper moves from `Lx/b;->a(I)I` to `Ly/t;->b(I)I` and changes 15 to 7. Each route has unchanged instructions under its actual target correspondence. Its resulting value changes inside its helper.
- Pairing equal helper bodies across the two unrelated routes gives the opposite history and invents two library-call changes. Both full old/new identities and the actual changed operations are required to distinguish these hypotheses.

## Preparation and reproduction

Use the [shared assembly procedure](../renamed-dispatch/#preparation-and-inspection) with `slug=swapped-helper-constants`. The two prepared DEX inputs were assembled with Smali 3.0.9, OpenJDK 21.0.12.1 and API 24. The before and after roles remain separate.

```sh
diana diff quality/fixtures/comparison/swapped-helper-constants/before/classes.dex \
  quality/fixtures/comparison/swapped-helper-constants/after/classes.dex
cargo nextest run --locked -p diana --test diff_test -E 'test(switched_constants)'
```

The owner-seeding matcher on base 7be3f1e paired all four methods across the wrong owners. It paired the two identical old/new helper bodies first and reported external API substitutions in both routes. The [semantic review](review.md) follows the independently authored data flow. [Method migration](../cross-owner-helper-move/) challenges the same use of owner and caller evidence from the other direction.

On the revised matcher based on 7be3f1e the CLI regression above passed on the same prepared inputs. The check executes the comparison and inspects the original member pairs and changed DEX sites.
