# Response snapshots, ordinary finally and suppressed close failures

Independently authored from R2 in
[the APK review](../../../reports/antennapod-3.12.3.md), without copied code.
The repository license applies. Java explains Case.smali and Handle.smali.
Owner/subjects: `Lfixture/ResponseClose;->response(I)I` and `resource(I)I`.

## Trigger and independent expectations

Recorded before first inspection. `response` merges acquisition-time null with
an acquired handle at exception entries, logs ordinary IOException, rethrows
InterruptedIOException first, and closes through ordinary finally. `resource`
keeps a pending body failure primary when close fails. The target is retained
identity/order; explicit protection is permitted. Whether these reduced methods
reproduce the original natural-scope refusal is not assumed.

All native helpers have this explicit model: acquire records `(acquire,mode)`
then returns handle H with identity1000+mode unless a scenario says null/failure.
Handle.ready records `(ready,H)` and returns the selected boolean; Handle.read
records `(read,H)` and returns77; close records `(close,H)`; log records the exact
throwable identity. A selected helper may throw its specified throwable after
recording its event. Throwable.addSuppressed is modeled by its ordinary Java
contract, including self-suppression rejection. These are models, not executed
native implementations. No acquisition ID is substituted for H's identity.

| Subject/scenario | Result/throwable and required order |
|---|---|
| response(7), ready true, normal | `77`: acquire7, ready1007, read1007, close1007, return captured77. No later read is invented. |
| response(7), ready false | `-1`: acquire7, ready1007, close1007; no read. |
| acquire throws IOException A | log A, return-1; response was never assigned, so no close. |
| acquire throws InterruptedIOException I | rethrow the same I, no log/ready/close. |
| read throws IOException A | acquire7, ready1007, read1007, log A, close1007, return-1. |
| read throws InterruptedIOException I | acquire7, ready1007, read1007, close1007, throw same I; IOException handler does not consume it. |
| read throws Error E | close1007 then rethrow E; no log. |
| read throws IOException A and log throws B | close1007 then throw B; no retry of log/close. |
| ordinary finally close throws C after success or pending failure | C replaces result/previous throwable; close executes once, no addSuppressed. |
| acquire returns null | null receiver failure at ready, no close and no fabricated handle. |
| resource(7), normal | acquire7, read1007, close1007, return77; no ready call. |
| resource body throws A, close throws distinct B | addSuppressed(A,B), then throw original A. Both failures remain, with A primary. |
| resource body throws A, close succeeds | throw same A after close1007. |
| resource body succeeds, close throws B | throw B instead of returning77; no suppression. |
| resource acquire throws A | A propagates without read/close/suppression. |
| resource close throws the same object A as body | addSuppressed rejects self-suppression with IllegalArgumentException; do not silently manufacture a suppressed entry or return A. |

The resource null-body case also retains its original null failure; a null handle
is never closed. Further library, linkage and allocation behavior is unmodeled.

## Preparation and status

```sh
mkdir -p .cache/fixtures/response-close-snapshots
java -cp "$SMALI_CLASSPATH" org.jf.smali.Main assemble --api 24 \
  quality/fixtures/exceptions/response-close-snapshots/Case.smali \
  quality/fixtures/exceptions/response-close-snapshots/Handle.smali \
  -o .cache/fixtures/response-close-snapshots/classes.dex
cp .cache/fixtures/response-close-snapshots/classes.dex \
  quality/fixtures/exceptions/response-close-snapshots/classes.dex
```

Baseline df10b744985e3407f902442b8252fab100700aec. Observed assembly used
Smali2.5.2, OpenJDK21.0.12.1 and API24, not a version admission pin.
ResponseClose is class0; resource is method2 and response method3.
ViewHandle is class1 with native close/read/ready methods4/5/6.

```sh
cargo run -p diana-decomp --example dbg_shell --locked -- \
  --dex quality/fixtures/exceptions/response-close-snapshots/classes.dex \
  --exec '{"cmd":"p6","class":0,"method":3,"details":true}' \
  --exec '{"cmd":"p9-all","text_only":true}'
```

Baseline and scalar-rendering candidate both expose response's
ProtectionBoundary graph with null/acquired-handle snapshots, and resource's
explicit primary/close/suppression chain. Their text and every P2–P8 product
are unchanged. The two preservation checks in
`crates/diana-decomp/tests/scalar_use_views.rs` pass on both revisions.
[review.md](review.md) records primary/secondary identity and fault-path reading.
This reproduces the readability limitation without inventing a behavioral
failure. That scalar repair did not recover natural protection.

## P6 recovery

Compared the same DEX on 73fc8793a91d766d021d117aa83ddfd644f7ac82 and the P6
repair. Response now has natural enclosing all/typed catches without the
ProtectionBoundary graph. Its normal and IO close suffixes remain outside the
protected body; canonical cleanup remains in the original all-handler. Resource
text is unchanged, including distinct primary/secondary reads and suppression.
P2–P5 and original P6 coordinates remain exact. Existing preservation checks and
the new scope/outlet check pass. Fresh [dual cleanup](../cleanup-outlet-frontiers/README.md)
and [shared contexts](../shared-cleanup-contexts/README.md) challenge identity,
normal outlets, terminal rethrows and outside recovery. Full isolated default/
all-feature runs pass 155 checks. This is not executed fixture behavior or an
unproved rewrite of ordinary finally into resource suppression.
