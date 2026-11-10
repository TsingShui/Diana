# Protocols through the ordinary control grammar

Project-authored Apache-2.0 challenge to
[protocol-owned-reads](../protocol-owned-reads/README.md).
Baseline: `5e01494`. The new interaction combines a single SuspendLambda reentry,
effectful argument preparation before state publication, normal-result copy
publication, a negative guard and an observed construction in the shared tail.
The marker/label prefix spans several original-operation portions. Those portions
must retain their existing entry/completion relation; a producer must not require
them to remain one obsolete coarse block.

Original selector: Lfixture/UnifiedBody;,
invokeSuspend(Ljava/lang/Object;)Ljava/lang/Object;. The Java explanation is the
logical initial body. Smali also fixes state dispatch, marker return, resumed
failure checking and invalid-state construction. The original ABI member and its
Object parameter remain present.

## Independent scenarios

Prepare records P and returns 7. Load records L(7), then returns a Number reply,
the distinct suspended marker, or a fault. Record records R(n). ReadBox allocation
records A, and its constructor stores first then second. Observe records O(a,b).
Boxing(-1) returns an Integer containing -1. Class initialization, superclass
construction and the marker helper are assumed successful; unmodeled runtime
faults remain outside these scenario models. The input alone supplies all engine
facts; these explanations are not external declaration inputs.

| Path | Required observation |
|---|---|
| Initial reply 5 | Return the same reply; P, L(7), R(5), A, O(5,5). |
| Initial reply -2 | Return Integer(-1); P, L(7), R(-2); no box allocation or observe. |
| Initial load faults | P, L(7), fault; no record, box or observe. |
| Initial load suspends | P, L(7), marker return; no shared-tail work yet. |
| Resume state 1 with reply 5 | Failure check, R(5), A, O(5,5), same reply return; no prepare/load repetition. |
| Resume state 1 with a failure | Propagate before record, construction or observe. |
| Invalid state | IllegalStateException construction and throw; no source-body helpers. |
| Box allocation faults after reply 5 | P, L(7), R(5), failed A; no observe or return. |

The selected source grammar must have one static owner per original executable
leaf, preserve every original call/result/terminal identity and supply a single
successful result publication. ABI and origin attachments cannot execute the
shared tail. The state store remains between Prepare and Load, even when its
text is an ABI comment. Normal and resumed delivery use the same existing storage.
These expectations precede candidate inspection. Coroutine/VM execution is unrun.

## Preparation

Assemble with Smali into `.cache/fixtures/protocol-common-controls/classes.dex`,
then publish that successful input beside its sources. Use the existing dbg_shell
inspection command with `--dex quality/fixtures/kotlin/protocol-common-controls/classes.dex`
and `--exec '{"cmd":"p9-all","text_only":true}'` on both revisions.

Prepared with Smali 2.5.2, API 24 and Temurin OpenJDK 21.0.12.1+1:

```sh
mkdir -p .cache/fixtures/protocol-common-controls
java -cp "$DIANA_SMALI_CLASSPATH" org.jf.smali.Main assemble --api 24 \
  -o .cache/fixtures/protocol-common-controls/classes.dex quality/fixtures/kotlin/protocol-common-controls
cp .cache/fixtures/protocol-common-controls/classes.dex quality/fixtures/kotlin/protocol-common-controls/classes.dex
```

Before/after inspection retains two code and four native/no-code methods with
no construction stops or rendering issues. The baseline establishes a local
suspension association but cannot select this source body; the replacement
selects ordinary controls with one successful publication. The paired
protocol_common_controls tests inspect ownership, original origins, publication
and state ordering. These structural observations supplement the path review;
coroutine/VM execution is unrun.
The two new product checks and the 42-test scoped nextest set pass. Strict engine,
dbg_shell and relevant-test Clippy passes with all features. The scoped result
is not a claim about executed coroutine/VM behavior or the full historical suite.
