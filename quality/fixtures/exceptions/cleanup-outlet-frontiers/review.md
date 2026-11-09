# Cleanup outlet frontier review

## Verdict

Compared the same prepared DEX on 73fc8793a91d766d021d117aa83ddfd644f7ac82
and the P6 repair. Direct LLM reading covers the explanation, actual Smali,
pre-inspection scenarios and complete subject output. Dual gains three natural
owning catches without acquiring its normal closes. Gap stays unchanged and
explicit. No contradiction was identified under the README native model; this
is not execution of those models or a claim about unspecified native behavior.

## Dual acquisition and nested recovery

The first acquisition commits A before the second begins. Failure before A's
commit saves null/null. Failure while acquiring B saves A/null, never an absent
B. After B commits, ready/read/log failures preserve A/B. The supplied original
receives keep these identities even when interruption rethrows through the outer
all-catch or a logging failure replaces the selected IOException.

When both handles are ready, readA commits10 and readB adds20 before the result30
is captured for normal cleanup. Neither ready returns-1; only B ready returns19,
not20. RuntimeException from readB leaves that call's result absent, logs its
original identity and adds1 to the preceding result, producing11 after readA.
Failure in that inner log can select the outer IOException recovery; its second
log then precedes cleanup and return-2. An unmatched log failure instead selects
the all-catch and remains pending through successful cleanup.

Every cleanup route closes B before A, conditionally on each original receiver.
A throwing closeB stops the route before A. CloseA failure replaces the pending
result/throwable after B succeeds. There is no resource-style suppression or
close retry. The normal and IO-recovery close suffixes are outside the all-catch
body. Their null-skipping paths jump to the original later suffix labels; they
do not fall into an unrelated close. Canonical all-catch closes stay in that
handler, not the protected body. All original calls/captures retain one anchor.

## Gap refusal

Acquire and read use the same original handlers, but probe between them is
unprotected. Probe failure O or E must escape without log/close. A successful
probe followed by read failure still selects the original IO or all-catch,
closing A once when present. If that ordinary-finally close fails, its failure
replaces the primary; no suppression is fabricated. The candidate retains the
explicit chains. It neither retries admission from read nor duplicates a shared
handler to hide the gap.

## Evidence

Two ordinary-Engine checks require dual's owning scopes, distinct acquisition
commits/snapshots, original close ordering and gap's unprotected call. The
iteration's nine checks pass on the candidate; fixed-baseline sensitivity is
two passes/seven failures. Both full isolated configurations pass 155 checks.
P2–P5 and original P6 nodes, links, clauses, completions, copies and locks agree
across 111 fixtures. Product/origin assertions support the reading; no callback,
APK/device, Java-output or full-method execution occurred.
