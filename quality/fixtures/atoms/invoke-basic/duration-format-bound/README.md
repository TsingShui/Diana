# Duration bound before formatting

This opt-in execution model selects
`Lorg/schabi/newpipe/util/Localization;->getDurationString(J)Ljava/lang/String;`
at `e4:l0:c7707:m56282` in the F-Droid NewPipe 0.28.8 APK. The pinned Java
source and DEX invoke instructions establish that a long argument is bounded by
zero before it is sent to the elapsed-time formatter.
[NewPipe case](../../../../decbench/corpora/newpipe-0.28.8.cases.json)

`DurationFormatModel.java` supplies an independently authored formatter stub.
The stub records each input and returns a deterministic token carrying that
input. The harness records the selected method's return after the formatter
events. It starts a fresh JVM with an empty event list for each argument in
[`model.json`](model.json).

| Input | Formatter input | Return token |
| --- | --- | --- |
| `-31` or `Long.MIN_VALUE` | `0` | `elapsed:0` |
| `0` | `0` | `elapsed:0` |
| `3661` | `3661` | `elapsed:3661` |
| `Long.MAX_VALUE` | `Long.MAX_VALUE` | `elapsed:9223372036854775807` |

Every scenario requires exactly one formatter event followed by the return.
[`ExtraFormatterCall.java`](ExtraFormatterCall.java) preserves the return value
and deliberately adds a formatter call before applying the bound. It is an
authored comparison control for the same method selector.

```sh
python3 quality/decbench prepare --corpus newpipe-0.28.8 --download
python3 quality/decbench run --corpus newpipe-0.28.8 --at e4:l0:c7707:m56282 \
  --binary target/debug/diana --execute-models
python3 quality/decbench run --corpus newpipe-0.28.8 --at e4:l0:c7707:m56282 \
  --jadx-jar /path/to/jadx-1.5.7-all.jar --execute-models
```

OpenJDK 21.0.12.1 executed all five scenarios against the pinned source, Diana
0.4.2 at `0df7f09c0207`, and JADX 1.5.7. Each matched all recorded expectations.
The altered method failed all five scenarios because of the extra formatter
event. [Semantic review](review.md)
