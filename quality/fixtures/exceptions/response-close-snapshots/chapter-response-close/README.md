# Chapter response cleanup model

This DecBench model selects the private AntennaPod 3.12.3
`ChapterUtils.loadChaptersFromUrl(String, CacheControl)` implementation at
`e5:l0:c462:m3263`. The
[public corpus cases](../../../../decbench/corpora/antennapod-3.12.3.cases.json)
record the APK digest, selected DEX method and pinned source span. The original
DEX invokes `Response.close()` at code units 54, 66 and 106. The upstream
method used as the execution control comes from the prepared source archive.
[Original method](https://github.com/AntennaPod/AntennaPod/blob/175e5ef8fe3e125a5014d70ef0c57cf85661c71b/ui/chapters/src/main/java/de/danoeh/antennapod/ui/chapters/ChapterUtils.java#L161-L180)

The template records operations on the request policy, the received response
and its body. Each close records the response identity. The return observation
distinguishes the parser's list object from a null result or another object.
Faults preserve throwable identity, and the final observation records the
suppressed-exception count. The executable links recording stubs and the Java
runtime. Its example URL is an argument recorded locally.

The expected traces in [model.json](model.json) follow the upstream method's
try, catch and finally flow.

| Scenario | Required observable outcome |
| --- | --- |
| Successful response | Read and parse the acquired response, close it once and return the parser's list. |
| Unsuccessful response | Close the acquired response once and return null. |
| Acquisition `IOException` | Log the failure and return null, with no acquired response. |
| Acquisition interruption | Propagate the original interruption, with no acquired response. |
| Read `IOException` | Log the failure, close the acquired response once and return null. |
| Read interruption | Close the acquired response once and propagate the original interruption. |
| Parser or logger failure | Close the acquired response once and propagate the original unchecked failure. |
| Close failure | Propagate the close failure without suppression when a list, a logged I/O failure or an interruption was pending. |

The pinned original and the Diana 0.4.2 publication passed all 11 scenarios.
The JADX 1.5.7 publication produced `?? r0 = 0;` in the selected method, so
compilation left all 11 scenarios unchecked. These are model observations for
the specified stubs and scenarios.

The qualification controls changed only the cleanup statement in the source
method supplied to the model. Closing a distinct response failed all nine
scenarios that acquired a response. Repeating the close failed all six
scenarios whose first close returned normally. Omitting cleanup only when the
body read raised `IOException` failed that scenario and preserved the other
ten results.

```sh
python3 quality/decbench run --corpus antennapod-3.12.3 \
  --binary target/debug/diana --execute-models --at e5:l0:c462:m3263
```

## Original DEX execution witness

[dex_probe.py](dex_probe.py) binds the same scenario inputs and event names
to exact original call targets. It reads the selected physical code item from
the SHA-256-pinned APK. The fixture stubs supply the state of the request,
response, response body and throwable. The witness executes the selected
branch, protected call and handler instructions under those stubs and records
original code-unit positions.

The raw DEX control passed all 11 authored scenarios. An acquisition
`IOException` at code unit 26 enters handler 72 without a response to close.
An interrupted read at code unit 46 enters handler 63, rethrows through the
cleanup handler and closes the acquired response at code unit 106. A close
failure at 106 escapes in place of that interruption. The source, DEX and
Diana executions agree under the same model assumptions. The raw witness
establishes its selected original control and effects under those assumptions.
