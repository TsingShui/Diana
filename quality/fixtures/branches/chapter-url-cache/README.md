# Chapter URL cache selection model

This model selects AntennaPod 3.12.3
`ChapterUtils.loadChaptersFromUrl(String, boolean)` at `e5:l0:c462:m3264`.
The [public corpus cases](../../../decbench/corpora/antennapod-3.12.3.cases.json)
record its APK digest, original method and pinned source location.
[Original method](https://github.com/AntennaPod/AntennaPod/blob/175e5ef8fe3e125a5014d70ef0c57cf85661c71b/ui/chapters/src/main/java/de/danoeh/antennapod/ui/chapters/ChapterUtils.java#L149-L159)

[model.json](model.json) states the expected input paths and traces. The Java
template and [raw DEX probe](dex_probe.py) bind the same symbolic cache
policies, list identities and interruption to recording stubs. The method uses
the supplied URL only as a recorded argument. Both harnesses execute with local,
authored behavior for the request and cache operations.

A forced refresh uses the network exclusively. In the regular path,
a missing cached result skips its size check. Cache sizes zero and one
fall back to the network, and size two returns the same cached list. A
throw during a cache or network request propagates the original interruption
without an additional request.

The original raw DEX witness confirms the null branch at code unit 15,
the size comparison at 22, and network fallback at 28. A usable cache goes
from 22 directly to return at 25. It records exact invocation targets, the
chosen policy, the result identity and the interruption escape. Its selected
scenario traces agree with the pinned source.

Diana 0.4.2 and JADX 1.5.7 published methods passed all eight executions
under the same stubs. An authored control changed the cache threshold to
treat a single result as usable. It failed the singleton and interrupted
fallback scenarios and passed the other six.

The raw interpreter supports bounded static methods with narrow scalar and
modeled reference values, selected forward and backward branches, ordinary
stub invocations, and ordered typed or catch-all handlers. Unsupported
reachable operations, unbound references and exhausted step budgets leave
the affected scenario unchecked.
