# JVM hierarchy and call-site navigation

`Relations.java` declares a nested-archive receiver, an inherited static
method and an inherited default interface method. A separate call chain
contains two sites that reference the same method, a self-call and three
distinct caller depths. `Diamonds.java` adds a second interface path to the
same ancestor with a more specific declaration on one path.

The preparation script compiles Java 17 class files with the local OpenJDK,
runs the six independent behavior checks in `Check.java`, and creates the
JAR layouts below. The check class stays outside the analyzed archives. The
local verification used OpenJDK 21.0.12.1 and Python 3.13.

- `relations.jar` has `Relations`, `Signal`, `Child`, `Base`, `Middle`,
  and `Terminal` at `j1` through `j6`. The Fabric declaration at `j0`
  identifies the archive at `j7`, which contains `Leaf` at `j7/j0`.
- `duplicate.jar` adds another physical `Terminal` definition at `j8`.
  `damaged.jar` instead adds a truncated class at `j8`.
- `diamond.jar` has `Diamonds`, `Common`, `Left`, `Right`, `Tip` and
  `Winning` at `j0` through `j5`.

```sh
python3 quality/fixtures/jvm/relations/prepare.py .cache/fixtures/jvm-relations
cargo nextest run -p diana --test jvm_relations_test --locked
```
