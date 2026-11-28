# JVM implicit and explicit monitor contract

The tested class is `MonitorContract.java`, compiled normally with
OpenJDK 21.0.12.1. The unchanged `CheckMonitorContract.java` is an independent
runner. Only the exact complete publication of `MonitorContract` is
recompiled with this runner.

The normal and throwing static and instance methods declare synchronization
and begin with a second explicit monitor on the detached gate. The runner
expects both monitors to be held during the body and neither to be held after
return or throw. The throwing methods preserve the original exception object.
The same-monitor nested cases check holding and release independently from
the number of raw explicit acquisitions. Native methods are declaration
controls checked by reflection and never invoked.

Raw `javap` flags and instructions are counted separately. The original
declared monitor has `ACC_SYNCHRONIZED` but no associated entry opcode; each
explicit block has an actual `monitorenter`. `nestedExplicit` is an
unsynchronized method with two explicit blocks on the same formal parameter.
The raw evidence counts both even when a runtime lock-presence query cannot
distinguish them.

On frozen Diana 0.4.14 the original passes all 28 runtime and reflection
checks and all 11 raw shape checks. The exact complete publication passes
24/28 and 5/11 after recompiling. Both distinct monitor variants report just
the implicit monitor in normal and faulting bodies. The explicit same-monitor
reentry in a synchronized method is absent, and the nested explicit-only
method has one acquisition instead of two. Implicit declarations,
explicit-only single acquire, and native declarations are controls.
The 0.4.15 candidate passes all 39 checks. The bytecode comparison covers
both entry and exit sites, and the executed paths check unlock behavior
after normal return and propagation of the same exception object.

```sh
fixture=quality/fixtures/sync/monitor-execution-contract/jvm
python3 -B "$fixture/verify.py" \
  --binary .cache/decbench-continuous/diana-0.4.14 \
  --output /tmp/diana-monitor-jvm-baseline.json
python3 -B "$fixture/verify.py" \
  --binary .cache/decbench-final-check-target/debug/diana \
  --output /tmp/diana-monitor-jvm-candidate.json
```

The baseline command exits 1 for its confirmed discrepancies. Both commands
save exact class publications, original and rebuilt javap listings, runtime
traces and input and binary digests.
