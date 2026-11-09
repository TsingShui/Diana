# Cleanup outlets with nested recovery and two handle identities

Independently authored fresh R2 challenge, after the draft P6 frontier repair and
before its first inspection. Parent:
[response-close-snapshots](../response-close-snapshots/README.md). Repository
license applies. Case.java explains the bytecode, not Engine declarations.

Original owners: `Lfixture/CleanupFrontiers;`, `Lfixture/FrontierHandle;`.
Subjects: `dual(I)I`, `gap(I)I`. The parent trigger is retained: protected seeds
have several unguarded normal completion suffixes and no shared normal follow.
Added interactions: two acquisition commits/identities, reverse close order,
inner RuntimeException recovery, int result phis/copies and skipped-close joins.
Gap deliberately reenters the same guarded layer after an unguarded effect;
it is a real refusal of a broad try, not an expected recovery failure to hide.

## Independent models and scenarios

Established before inspecting output. Native acquire(mode,ordinal) records
acquire1/acquire2 then yields distinct handles A/B. ready records readyA/readyB and
returns the selected boolean. read records readA/readB then returns10/20. log
records log(T), close records closeA/closeB, probe records probe. A scenario can
instead throw a specified identity at any recorded call before its result commit.
I is InterruptedIOException, O is IOException excluding I, R is RuntimeException,
and E/F are otherwise unmatched Throwable identities. This closed test model does
not establish external declarations or behavior for Engine; no model is executed.

| Scenario | Ordered result/effect obligations |
|---|---|
| dual, both ready | acquire1, acquire2, readyA, readA, readyB, readB, closeB, closeA; return30. |
| Neither ready | acquire1, acquire2, readyA, readyB, closeB, closeA; return-1. |
| Only B ready | acquire1, acquire2, readyA, readyB, readB, closeB, closeA; return19, not20. |
| acquire1 throws O | log(O), then return-2; both cleanup snapshots are null, no close. |
| acquire2 throws O | log(O), closeA, return-2; failed B acquisition never becomes a cleanup receiver. |
| acquire2 throws I | no log; closeA then propagate identical I. |
| readA throws O | log(O), closeB, closeA, return-2; no readyB/readB or new result commit. |
| readB throws R after readA=10 | log(R), closeB, closeA; return11, not30; failed read result is absent. |
| inner log(R) throws O | outer IO recovery runs log(O), then closeB, closeA and return-2. |
| inner log(R) throws E | closeB, closeA, propagate E; no fabricated suppression. |
| IO log(O) throws E | catch-all cleanup closes B then A, then propagates E. |
| closeB throws F on normal or recovery route | F propagates; closeA does NOT execute, result/pending throwable is replaced. |
| closeA throws F after closeB succeeds | F propagates after exactly one closeB/closeA; no close retry. |
| gap, ordinary success | acquire1, probe, readA, closeA; return10. |
| gap, probe throws O or E | acquire1, probe, then identical failure propagates; no log or close. A broad try would be incorrect. |
| gap, readA throws O | acquire1, probe, readA, log(O), closeA, return-2. |
| gap, readA throws E; closeA throws F | acquire1, probe, readA, closeA; F replaces E, no suppression. |

Require natural nested owning catch scopes for dual without acquiring any normal
close suffix. Every captured A/B snapshot, ordered clause and partial int commit
must remain original. B is closed before A, and failure stops that route. No
resource-style addSuppressed is introduced into this ordinary-finally case.
Existing resource suppression semantics remain separately guarded by the parent.
Gap must keep probe outside every catch scope; sharing cleanup cannot license a
second-start search, duplicated handler or dispatcher.

## Preparation and evidence

```sh
mkdir -p .cache/fixtures/cleanup-outlet-frontiers
java -cp "$SMALI_CLASSPATH" org.jf.smali.Main assemble --api 24 \
  quality/fixtures/exceptions/cleanup-outlet-frontiers/Case.smali \
  quality/fixtures/exceptions/cleanup-outlet-frontiers/Handle.smali \
  -o .cache/fixtures/cleanup-outlet-frontiers/classes.dex
cp .cache/fixtures/cleanup-outlet-frontiers/classes.dex \
  quality/fixtures/exceptions/cleanup-outlet-frontiers/classes.dex
```

Baseline 73fc8793a91d766d021d117aa83ddfd644f7ac82. Actual assembly used Smali2.5.2,
OpenJDK21.0.12.1 and API24, not an exact-version admission pin. CleanupFrontiers
is class0: dual/method1, gap/method2. FrontierHandle is class1, with native
close/read/ready methods5/6/7. Dual gains three natural catch scopes; gap's text
is unchanged. [review.md](review.md) records the semantic reading.
Nine iteration checks pass after two passes/seven failures on the baseline;
full isolated default/all-feature runs pass 155 checks. None executes the model.
