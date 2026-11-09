# Fixture: terminal-follow-conjunction

This fixture reproduces an ordered short-circuit conjunction whose selected
failure path is also the condition's common follow. It is an independently
authored reduction of the `equals` shape found in `okhttp3.Route`; no private
application source, mapping or data is included.

## Original input

- Owner: `Lfixture/TerminalFollow;`
- `matches(Ljava/lang/Object;)Z`
- `effectful(Ljava/lang/Object;)Z`

The prepared DEX is generated from `TerminalFollow.java` with the repository's
shared Java 8 and D8 settings. The class has three reference fields and a static
`eq` helper. Each subject tests `instanceof`, casts the receiver, reads fields
and compares them in left-to-right order. A failed test jumps directly to the
common failure block, which prints or returns and is therefore an abrupt follow.

## Independent expectations

`matches(other)` returns `true` exactly when `other` is a
`TerminalFollow` and all three fields equal the receiver's fields. A null or
foreign receiver returns `false` before any field read. The three `eq` calls run
in declaration order and stop at the first false result.

`effectful(other)` tests `instanceof`, then `probe(1)`, then the first field
equality, then `probe(2)`. On the true path it calls `mark(10)` and returns
`true`. Every failure path calls `mark(20)` and returns `false`. A skipped later
test is not called. The receiver cast and field reads occur only after the
`instanceof` test succeeds.

## Preparation

Prepared with `javac 21.0.12.1` and `D8 9.2.4-dev` from Android build-tools
37.0.0. The command uses the environment's SDK location rather than a fixed
machine path:

```sh
mkdir -p .cache/fixtures/terminal-follow-conjunction/classes
mkdir -p .cache/fixtures/terminal-follow-conjunction/dex
javac --release 8 -g -d .cache/fixtures/terminal-follow-conjunction/classes \
  quality/fixtures/branches/terminal-follow-conjunction/TerminalFollow.java
$ANDROID_HOME/build-tools/37.0.0/d8 --min-api 24 \
  --output .cache/fixtures/terminal-follow-conjunction/dex \
  .cache/fixtures/terminal-follow-conjunction/classes/fixture/TerminalFollow.class
cp .cache/fixtures/terminal-follow-conjunction/dex/classes.dex \
  quality/fixtures/branches/terminal-follow-conjunction/classes.dex
```

Inspect with the ordinary engine entry:

```sh
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/branches/terminal-follow-conjunction/classes.dex \
  --exec '{"cmd":"p9-all","text_only":true}'
```

## Reproduction status

The pre-fix candidate on the discovery input emitted nested `If` regions and no
`ConditionPlan` for the conjunction. The repaired candidate is expected to emit
one ordered condition for each subject while retaining the original branch
edges, casts, calls and result arms. This README records independent behavior;
the written review and engine tests are separate evidence.

The candidate emits one condition for each subject. `matches` evaluates the
three field tests left to right and returns `false` from the shared terminal arm;
`effectful` keeps `probe(1)`, the field equality and `probe(2)` in that order,
with `mark(20)` only on failure and `mark(10)` only on success. The P4 proof
removes the three redundant casts after the successful `instanceof`; the source
edge identities remain retained in the engine product.
