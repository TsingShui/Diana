"""Same recording model for the selected original AntennaPod DEX instructions."""

import json
from pathlib import Path

from dex_witness import Fault, Raised, Ref, Scenario, Unavailable, evaluate

INPUT_SHA256 = "6e79f28f69cef1a8b2e4f5f88c9c3cf3c3d7c51356f01394ac23b0dc5d56a00b"
OWNER = "Lde/danoeh/antennapod/ui/chapters/ChapterUtils;"
AT = "e5:l0:c462:m3263"
MEMBER = OWNER + "->loadChaptersFromUrl(Ljava/lang/String;Lokhttp3/CacheControl;)Ljava/util/List;"
URL = "https://fixture.invalid/chapters"
CACHE = Ref("cache", "Lokhttp3/CacheControl;")
CHAPTERS = Ref("chapters", "Ljava/util/List;")
BUILDER = "Lokhttp3/Request$Builder;"
STRING_BUILDER = "Ljava/lang/StringBuilder;"
REQUEST = Ref("request", "Lokhttp3/Request;")
CLIENT = Ref("client", "Lokhttp3/OkHttpClient;")
CALL = Ref("call", "Lokhttp3/Call;")
RESPONSE = Ref("acquired", "Lokhttp3/Response;")
BODY = Ref("acquired", "Lokhttp3/ResponseBody;")
EXCEPTION = "Ljava/lang/Exception;"
THROWABLE = "Ljava/lang/Throwable;"
OBJECT = "Ljava/lang/Object;"
RUNTIME = "Ljava/lang/RuntimeException;"
IO = Fault("io", "Ljava/io/IOException;", (EXCEPTION, THROWABLE, OBJECT))
INTERRUPTED = Fault("interrupted", "Ljava/io/InterruptedIOException;", (IO.descriptor, EXCEPTION, THROWABLE, OBJECT))
PARSE, CLOSE, LOGGER = [
    Fault(name, "Ljava/lang/IllegalStateException;", (RUNTIME, EXCEPTION, THROWABLE, OBJECT))
    for name in ("parser", "close", "logger")
]
HTTP_CLIENT = "Lde/danoeh/antennapod/net/common/AntennapodHttpClient;"
PARSER = "Lde/danoeh/antennapod/parser/feed/PodcastIndexChapterParser;"


class ChapterResponse:
    def __init__(self, scenario):
        self.scenario = scenario
        self.events = []
        self.objects = {}
        self.calls = {
            (BUILDER, "<init>", "()V"): self.builder,
            (BUILDER, "url", "(Ljava/lang/String;)" + BUILDER): self.url,
            (BUILDER, "cacheControl", "(" + CACHE.descriptor + ")" + BUILDER): self.cache,
            (BUILDER, "build", "()" + REQUEST.descriptor): self.build,
            (HTTP_CLIENT, "getHttpClient", "()" + CLIENT.descriptor): self.client,
            (CLIENT.descriptor, "newCall", "(" + REQUEST.descriptor + ")" + CALL.descriptor): self.new_call,
            (CALL.descriptor, "execute", "()" + RESPONSE.descriptor): self.execute,
            (RESPONSE.descriptor, "isSuccessful", "()Z"): self.status,
            (RESPONSE.descriptor, "body", "()" + BODY.descriptor): self.body,
            (BODY.descriptor, "string", "()Ljava/lang/String;"): self.read,
            (PARSER, "parse", "(Ljava/lang/String;)" + CHAPTERS.descriptor): self.parse,
            (RESPONSE.descriptor, "close", "()V"): self.close,
            (STRING_BUILDER, "<init>", "()V"): self.string_builder,
            (STRING_BUILDER, "append", "(Ljava/lang/String;)" + STRING_BUILDER): self.append,
            (STRING_BUILDER, "toString", "()Ljava/lang/String;"): self.string,
            ("Landroid/util/Log;", "d", "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/Throwable;)I"): self.log,
        }

    def static_field(self, member):
        raise Unavailable(f"response model has no static field binding for {member}")

    def allocate(self, owner):
        if owner not in (BUILDER, STRING_BUILDER):
            raise Unavailable(f"response model cannot allocate {owner}")
        return Ref(f"allocation-{len(self.objects)}", owner)

    def call(self, member, receiver, arguments):
        callee = self.calls.get(member)
        if callee is None:
            raise Unavailable(f"response model has no exact call binding for {member}")
        return callee(receiver, *arguments)

    def builder(self, receiver):
        self.events.append("request-builder")
        self.objects[receiver] = {}

    def url(self, receiver, value):
        self.events.append("request-url" if value == URL else "wrong-url")
        self.objects[receiver]["url"] = value
        return receiver

    def cache(self, receiver, value):
        self.events.append("request-cache" if value == CACHE else "wrong-cache")
        self.objects[receiver]["cache"] = value
        return receiver

    def build(self, receiver):
        self.events.append("request-build")
        self.objects[REQUEST] = self.objects[receiver].copy()
        return REQUEST

    def client(self, _):
        self.events.append("client")
        return CLIENT

    def new_call(self, receiver, request):
        state = self.objects.get(request, {})
        correct = receiver == CLIENT and state.get("url") == URL and state.get("cache") == CACHE
        self.events.append("new-call" if correct else "wrong-request")
        return CALL

    def execute(self, receiver):
        if receiver != CALL:
            raise Unavailable("execute receiver is outside the response model")
        self.events.append("execute")
        if self.scenario == "EXECUTE_IO":
            raise Raised(IO)
        if self.scenario == "EXECUTE_INTERRUPTED":
            raise Raised(INTERRUPTED)
        self.events.append("acquire-response")
        return RESPONSE

    def status(self, receiver):
        self.events.append("status-" + receiver.name)
        return int(self.scenario != "UNSUCCESSFUL")

    def body(self, receiver):
        self.events.append("body-" + receiver.name)
        return Ref(receiver.name, BODY.descriptor)

    def read(self, receiver):
        self.events.append("read-" + receiver.name)
        if self.scenario in {"READ_IO", "READ_IO_CLOSE_FAILURE", "LOG_FAILURE"}:
            raise Raised(IO)
        if self.scenario in {"READ_INTERRUPTED", "INTERRUPTED_CLOSE_FAILURE"}:
            raise Raised(INTERRUPTED)
        return receiver.name

    def parse(self, _, value):
        self.events.append("parse-" + value)
        if self.scenario == "PARSE_FAILURE":
            raise Raised(PARSE)
        return CHAPTERS

    def close(self, receiver):
        self.events.append("close-" + receiver.name)
        if self.scenario in {"SUCCESS_CLOSE_FAILURE", "READ_IO_CLOSE_FAILURE", "INTERRUPTED_CLOSE_FAILURE"}:
            raise Raised(CLOSE)

    def string_builder(self, receiver):
        self.objects[receiver] = ""

    def append(self, receiver, value):
        self.objects[receiver] += value
        return receiver

    def string(self, receiver):
        return self.objects[receiver]

    def log(self, _, tag, message, failure):
        if not isinstance(failure, Fault):
            raise Unavailable("response log has no established throwable")
        self.events.append("log-" + failure.name if tag == "ChapterUtils" and URL in message else "wrong-log")
        if self.scenario == "LOG_FAILURE":
            raise Raised(LOGGER)
        return 0

    def finish(self, value, fault):
        if fault is not None:
            self.events.append("throw-" + fault.name + "-suppressed-0")
        else:
            self.events.append("return-chapters" if value == CHAPTERS else
                               "return-null" if value == 0 else "return-other")


def scenarios():
    model = json.loads(Path(__file__).with_name("model.json").read_text(encoding="utf-8"))
    return [
        Scenario(row["id"], (URL, CACHE), lambda name=row["arguments"][0]: ChapterResponse(name),
                 tuple(row["expected"]))
        for row in model["scenarios"]
    ]


def probe(artifact, *, at=AT, member=MEMBER):
    if (at, member) != (AT, MEMBER):
        raise ValueError("the selected original does not match the authored raw DEX probe")
    return evaluate(artifact, input_sha256=INPUT_SHA256, at=at, member=member, scenarios=scenarios())
