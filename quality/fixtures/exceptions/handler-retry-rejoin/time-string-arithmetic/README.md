# Time string arithmetic and parser failures

This opt-in model selects
`Lde/danoeh/antennapod/parser/feed/util/DateUtils;->parseTimeString(Ljava/lang/String;)J`
at `e4:l0:c8337:m55147` in the pinned AntennaPod 3.12.3 release. The authored
case records the original method, source span and DEX parser call sites.
[AntennaPod cases](../../../../decbench/corpora/antennapod-3.12.3.cases.json)

The template runs each argument in a separate JVM. It records the returned
millisecond count or exception class. A `NumberFormatException` also records
which JDK parser threw, using the method's stack frame. The result contains no
localized exception messages. The parser operations use the JDK implementations.

| Input | Expected result or fault |
| --- | --- |
| `01:02:03.5` | `3723500` |
| `02:03.5` | `123500` |
| `not-a-number` | `0` |
| `::` | `0` |
| `1:2:ignored:ignored` | `62000` |
| `1:2:` | `62000` |
| `1::2` | Integer parser throws `NumberFormatException` |
| `-2:-3.5` | `-123500` |
| `2147483647:0` | `128849018820000` |
| `0:16777217` | `16777216000` |
| `0:-0.0009` | `0` |
| `1:NaN` | `60000` |
| `0:Infinity` | `9223372036854775807` |
| `0:-Infinity` | `-9223372036854775808` |
| `1:Infinity` | `-9223372036854715809` |
| `bad-minute:bad-second` | Integer parser throws `NumberFormatException` |
| `0:bad-second` | Float parser throws `NumberFormatException` |
| `bad-hour:1:bad-second` | Integer parser throws `NumberFormatException` |
| `null` | `NullPointerException` |

The [authored comparison control](WrongOrderAndScale.java) parses seconds before
minutes and casts to `long` before scaling seconds. A fault in both components
distinguishes evaluation order. Fractional values and infinities expose effects
of the early conversion.

```sh
python3 quality/decbench prepare --corpus antennapod-3.12.3 --download
python3 quality/decbench run --corpus antennapod-3.12.3 --at e4:l0:c8337:m55147 \
  --binary target/debug/diana --execute-models
python3 quality/decbench run --corpus antennapod-3.12.3 --at e4:l0:c8337:m55147 \
  --jadx-jar /path/to/jadx-1.5.7-all.jar --execute-models
```

All 19 scenarios matched the independent expectations on OpenJDK 21.0.12.1
with the pinned source, Diana 0.4.2, JADX 1.5.7, and Diana 0.1.0 at `b214ec60`.
The altered method failed seven scenarios. [Semantic review](review.md)
