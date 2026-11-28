# Chapter source order model

This DecBench model exercises the original `ChapterUtils.loadChapters`
definition from the pinned AntennaPod 3.12.3 corpus at `e5:l0:c462:m3261`.
The [corpus manifest](../../../../decbench/corpora/antennapod-3.12.3.cases.json)
records the APK, source and selected bytecode evidence.

The authored stubs represent a `FeedMedia` whose database and network chapter
sources are independently enabled. The model records database, network, media,
merge and setter events. Its interruption scenario throws before a network
result is delivered. Compare the recovered method and pinned source against
the same stubs and independently stated event traces in [model.json](model.json).
This tests selected interactions and effect order under those stubs.
The `TextUtils.isEmpty` stub uses the `CharSequence` parameter recorded in
the original DEX call descriptor.

The original source ran all four scenarios against the expected traces. On
Diana 0.4.2, the recovered method passed the two individual-source controls.
When both sources were present it called the network before the database. A
network interruption also skipped the database call. The recovered method from
JADX 1.5.7 passed all four scenarios. The original APK provides source and DEX
alignment. The execution uses the isolated model.

```sh
python3 quality/decbench run --corpus antennapod-3.12.3 \
  --binary target/debug/diana --execute-models --at e5:l0:c462:m3261
```
