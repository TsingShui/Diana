"""Compare modeled array allocation timing with the retained DEX caller."""

from hashlib import sha256
import json
from pathlib import Path
import runpy
from tempfile import TemporaryDirectory
from zipfile import ZipFile

from corpus_models import MARKER, Model, compare, method_source
from corpus_oracles import IDENTIFIER, Unavailable, View
from dex_witness import Scenario, evaluate
from evidence import file_digest, method


FIXTURE = Path(__file__).resolve().parent
ALLOCATORS = {"int": "allocateInts", "Object": "allocateObjects"}


def expressions(view, text, opening, closing):
    elements = []
    start = cursor = opening + 1
    while cursor < closing:
        if view.words[cursor] == ",":
            if start == cursor:
                raise Unavailable("empty array initializer element")
            elements.append(text[view.offsets[start][0]:view.offsets[cursor - 1][1]])
            start = cursor + 1
        cursor = view.ends.get(cursor, cursor) + 1
    if start < closing:
        elements.append(text[view.offsets[start][0]:view.offsets[closing - 1][1]])
    return elements


def instrument(body):
    """Insert the fault hook at Java's allocation point without moving operands."""
    view = View(body, locate_tokens=True)
    words, offsets = view.words, view.offsets
    allocations = [index for index in range(len(words) - 2)
                   if words[index] == "new" and words[index + 1] in ALLOCATORS and words[index + 2] == "["]
    if len(allocations) != 1:
        raise Unavailable("the selected method must expose one supported array creation")
    new, = allocations
    kind = words[new + 1]
    allocate = f"ArrayOrder.{ALLOCATORS[kind]}"
    dimension = new + 2
    closing = view.ends[dimension]
    if closing != dimension + 1:
        if words[closing + 1:closing + 2] != (";",):
            raise Unavailable("unsupported sized array context")
        length = body[offsets[dimension][1]:offsets[closing][0]]
        return body[:offsets[new][0]] + f"{allocate}({length})" + body[offsets[closing][1]:]

    opening = closing + 1
    if words[opening:opening + 1] != ("{",) or opening not in view.ends:
        raise Unavailable("the recovered initializer has no supported component list")
    closing = view.ends[opening]
    if words[closing + 1:closing + 2] != (";",):
        raise Unavailable("unsupported initializer context")
    start = new
    while start and words[start - 1] not in (";", "{", "}"):
        start -= 1
    context = words[start:new]
    is_local = (len(context) == 5 and context[:3] == (kind, "[", "]")
                and IDENTIFIER.fullmatch(context[3]) and context[4] == "=")
    if context != ("return",) and not is_local:
        raise Unavailable("initializer recovery needs a return or initialized local")
    name = "decbenchAllocatedArray"
    if name in words:
        raise Unavailable("the allocation witness local conflicts with existing code")
    values = expressions(view, body, opening, closing)
    if not 0 < len(values) <= 3:
        raise Unavailable("the recovered component list exceeds the fixture scope")
    prefix = f"{kind}[] {name} = {allocate}({len(values)});\n"
    prefix += "".join(f"{name}[{index}] = {value};\n" for index, value in enumerate(values))
    statement = body[offsets[start][0]:offsets[new][0]] + name
    return body[:offsets[start][0]] + prefix + statement + body[offsets[closing][1]:]


def probe(binary, artifact, timeout):
    manifest_path = FIXTURE / "decbench.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    scenarios_path = FIXTURE / "scenarios.json"
    scenarios = json.loads(scenarios_path.read_text(encoding="utf-8"))
    source = FIXTURE / "ArrayOrder.java"
    reference = source.read_text(encoding="utf-8")
    raw_model = runpy.run_path(str(FIXTURE / "raw_model.py"))["ArrayModel"]
    with artifact.open("rb") as stream:
        data = stream.read(64 * 1024 + 1)
    if len(data) > 64 * 1024 or sha256(data).hexdigest() != manifest["input_sha256"]:
        raise ValueError("the bounded fixture DEX differs from its manifest")

    observations = {}
    with TemporaryDirectory(prefix="decbench-filled-array-") as directory:
        archive = Path(directory) / "original.zip"
        with ZipFile(archive, "w") as writer:
            writer.writestr("classes.dex", data)
        original_sha = file_digest(archive)
        for case in manifest["methods"]:
            at, member = case["at"], case["member"]
            cases = scenarios[member.split("->", 1)[1].split("(", 1)[0]]
            original = method_source(reference, member)
            model = Model(scenarios_path, source, reference.replace(original, MARKER, 1), source.stem, cases, None)
            captured = method(binary, artifact, at, timeout, member=member)
            if captured.get("text"):
                try:
                    observed = method_source(captured["text"], member, require_statements=True)
                    captured = {**captured, "text": instrument(observed)}
                except Unavailable as error:
                    captured = {"error": str(error)}
            rows = compare(model, original, captured, member, timeout)
            raw = evaluate(archive, input_sha256=original_sha, at=at, member=member, scenarios=[
                Scenario(spec["id"], (), lambda fault=spec["arguments"][1]: raw_model(fault), tuple(spec["expected"]))
                for spec in cases
            ])
            for row, control in zip(rows, raw, strict=True):
                row["original_dex"] = control
                if control["status"] != "PASS":
                    row["status"] = control["status"]
                    row["detail"] = control["detail"]
            observations[at] = rows
    return observations
