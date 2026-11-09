# Shared cleanup contexts and resource completion outlets

Fresh, independently authored challenge after the R2 shared-frontier/terminal-seed
refinement, before first inspection. Parents:
[cleanup-outlet-frontiers](../cleanup-outlet-frontiers/README.md) and
[response-close-snapshots](../response-close-snapshots/README.md).
No upstream implementation is copied; repository license applies. Java explains
the paired Smali, not declarations supplied to Engine.

Original owners: `Lfixture/CleanupContexts;`, `Lfixture/ContextHandle;`.
Subjects: `split(I)I`, `resource(I)I`.

Split retains the missing-follow trigger but adds distinct pre/post-acquisition
captures, a shared pure cleanup guard, shared logging and interrupted rethrow.
The terminal rethrow has no normal successor, so the other seeds' apparent
normal follow is insufficient. The shared guard must stay outside both the body
and the selected catch, not be acquired as a private body reentry. Original post
entries can remain explicit where their crossing contexts do not supply a
natural scope; no capture/receiver alias is invented.

Resource adds two normal close outlets, a note that can fail after the read
commit, nested primary/secondary suppression and an outer IOException recovery
also used by acquisition. Cutting normal close effects must also remove exception
fragments reachable only through them from the proposed body. Otherwise that
outer recovery falsely looks like private resource-body content.

## Independent scenarios

Recorded before inspection. acquire records acquire then normally returns H.
ready records ready and normally returns true. read records read and returns10.
log records log(T), note records note(value), close records close(H).
Each call can instead throw a selected identity before its result commit. I is
InterruptedIOException; O is IOException excluding I; E/F are otherwise unmatched
Throwable identities. A scenario can choose false ready, negative read or a null
acquisition. Library suppression is modeled independently: addSuppressed retains
primary E and attaches distinct secondary F; E==F rejects self-suppression with S.
This is not an implementation supplied to Engine or an execution of the model.

| Scenario | Original result and event obligations |
|---|---|
| split normal | acquire, ready, read, close(H); return10. |
| split ready false | acquire, ready, close(H); return-1, no read. |
| split read=-3 | acquire, ready, read, close(H); return-1, not-3. |
| acquire throws O | acquire, log(O); return-1 with null receiver and no close. |
| acquire throws I | acquire, then identical I; no log or close. |
| ready/read throws O | log(O), close(H), return-1; no successful result use after the failure. |
| ready/read throws I | no log; close(H), then identical I through shared terminal rethrow. |
| log(O) throws E after acquisition failure | E propagates; the post capture's receiver is still null, no close. |
| log(O) throws E after read failure | close(H), then E, not O. |
| split close throws F | F replaces the pending return/throwable; no close retry or addSuppressed. |
| acquire yields null | ready fails with modeled NPE; both shared cleanup adapters retain null, no close, then NPE. |
| resource normal | acquire, read, note(10), close(H); return10. |
| resource read=-3 | acquire, read, close(H); return0, no note. |
| resource acquire throws O | acquire, log(O), return-7; no close. |
| resource normal close throws O | acquire, read, note(10), close(H), log(O), return-7; no second close or suppression. |
| resource read/note throws E, close succeeds | close(H), then identical E, no outer IO log. |
| resource read/note throws O, close succeeds | close(H), log(O), return-7; O is the original primary. |
| resource body E, close F, E!=F | one close; retain E with suppressed F, no replacement by F. |
| resource body O, close F, O!=F | one close; log original O with its F suppression, return-7. |
| resource body E, close same E | self-suppression rejects with S; S propagates, not fabricated success/E. |
| outer IO log throws F | F propagates without another close. |

Require natural enclosing protection for split, plus a natural resource body and
its nested close-failure catch for resource. Normal closes stay outside the
resource-body catch. Keep original entry identities, captures, receives, partial
result commits and normal/exceptional copy phases. Do not merge the two all
captures or infer receiver equality from a shared tail. The prior genuine foreign
entry and unguarded-gap refusals remain part of this iteration's regressions.

Native/linkage/allocation behavior outside this model is unknown. No fixture VM,
Java-output recompilation or device execution is performed.

## Preparation and evidence

```sh
mkdir -p .cache/fixtures/shared-cleanup-contexts
java -cp "$SMALI_CLASSPATH" org.jf.smali.Main assemble --api 24 \
  quality/fixtures/exceptions/shared-cleanup-contexts/Case.smali \
  quality/fixtures/exceptions/shared-cleanup-contexts/Handle.smali \
  -o .cache/fixtures/shared-cleanup-contexts/classes.dex
cp .cache/fixtures/shared-cleanup-contexts/classes.dex \
  quality/fixtures/exceptions/shared-cleanup-contexts/classes.dex
```

Baseline 73fc8793a91d766d021d117aa83ddfd644f7ac82. Actual assembly used Smali2.5.2,
OpenJDK21.0.12.1 and API24, not an exact-version admission pin. CleanupContexts
is class0: resource/method3, split/method4. ContextHandle is class1; native
close/read/ready are5/6/7. Split gains two natural scopes and resource gains its
missing body catch, for three nested scopes. Captures and shared cleanup remain
separate. [review.md](review.md) explains every scenario and rejected drafts.
The nine iteration checks give two passes/seven failures on the fixed baseline
and nine passes on the candidate. Both full isolated configurations pass 155 checks.
Across 111 fixtures/179 classes/853 methods, P2–P5 and original P6 coordinates
are unchanged, with no retrieval/render failures. These are not executions of
this fixture or its native/library models.
