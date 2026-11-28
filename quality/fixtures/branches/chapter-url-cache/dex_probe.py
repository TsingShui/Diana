"""Raw original DEX witness for cache and network guards in AntennaPod."""

import json
from pathlib import Path

from dex_witness import Fault, Raised, Ref, Scenario, Unavailable, evaluate

INPUT_SHA256 = "6e79f28f69cef1a8b2e4f5f88c9c3cf3c3d7c51356f01394ac23b0dc5d56a00b"
OWNER = "Lde/danoeh/antennapod/ui/chapters/ChapterUtils;"
AT = "e5:l0:c462:m3264"
MEMBER = OWNER + "->loadChaptersFromUrl(Ljava/lang/String;Z)Ljava/util/List;"
URL = "https://fixture.invalid/chapters"
POLICY = "Lokhttp3/CacheControl;"
LIST = "Ljava/util/List;"
FORCE_CACHE = Ref("force-cache", POLICY)
FORCE_NETWORK = Ref("force-network", POLICY)
CACHE = Ref("cache", LIST)
NETWORK = Ref("network", LIST)
REQUEST = (OWNER, "loadChaptersFromUrl", "(Ljava/lang/String;" + POLICY + ")" + LIST)
SIZE = (LIST, "size", "()I")
INTERRUPTED = Fault("interrupted", "Ljava/io/InterruptedIOException;", (
    "Ljava/io/IOException;", "Ljava/lang/Exception;", "Ljava/lang/Throwable;", "Ljava/lang/Object;",
))


class ChapterCache:
    def __init__(self, scenario):
        self.scenario = scenario
        self.events = []

    def static_field(self, member):
        if member == (POLICY, "FORCE_CACHE", POLICY):
            return FORCE_CACHE
        if member == (POLICY, "FORCE_NETWORK", POLICY):
            return FORCE_NETWORK
        raise Unavailable(f"cache policy field is outside the model: {member}")

    def allocate(self, owner):
        raise Unavailable(f"cache wrapper cannot allocate {owner}")

    def call(self, member, receiver, arguments):
        if member == SIZE and receiver == CACHE:
            self.events.append("cache-size")
            return {"EMPTY": 0, "TWO": 2}.get(self.scenario, 1)
        if member != REQUEST:
            raise Unavailable(f"cache wrapper has no exact call binding for {member}")
        url, policy = arguments
        if url != URL:
            self.events.append("wrong-url")
        if policy == FORCE_CACHE:
            self.events.append("cache")
            if self.scenario == "CACHE_INTERRUPTED":
                raise Raised(INTERRUPTED)
            return 0 if self.scenario == "MISSING" else CACHE
        if policy == FORCE_NETWORK:
            self.events.append("network")
            if self.scenario in {"FORCED_INTERRUPTED", "FALLBACK_INTERRUPTED"}:
                raise Raised(INTERRUPTED)
            return NETWORK
        self.events.append("wrong-policy")
        return 0

    def finish(self, value, fault):
        if fault is not None:
            self.events.append("throw-" + fault.name)
        else:
            expected = {CACHE: "return-cache", NETWORK: "return-network", 0: "return-null"}
            self.events.append(expected.get(value, "return-other"))


def scenarios():
    model = json.loads(Path(__file__).with_name("model.json").read_text(encoding="utf-8"))
    return [
        Scenario(row["id"], (URL, int(row["arguments"][0].startswith("FORCED"))),
                 lambda name=row["arguments"][0]: ChapterCache(name), tuple(row["expected"]))
        for row in model["scenarios"]
    ]


def probe(artifact, *, at=AT, member=MEMBER):
    if (at, member) != (AT, MEMBER):
        raise ValueError("the selected original does not match the authored raw DEX probe")
    return evaluate(artifact, input_sha256=INPUT_SHA256, at=at, member=member, scenarios=scenarios())
