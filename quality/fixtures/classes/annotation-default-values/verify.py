"""Read this fixture's DEX defaults and reflect on exact Java publications."""

from hashlib import sha1
import json
from pathlib import Path
from subprocess import SubprocessError
from tempfile import TemporaryDirectory
from zlib import adler32

from dex_truth import Dex, U16, U32
from tool_capture import run


ROOT = Path(__file__).resolve().parent
OWNERS = ("DexDefaults", "DexLabel")
DECLARING = "Lfixture/DexDefaults;"
CARRIER = "Ldalvik/annotation/AnnotationDefault;"
JAVA = ["java", "-XX:+IgnoreUnrecognizedVMOptions", "-XX:-UsePerfData", "-Xmx128m"]


def original_string(dex, index):
    offset = U32.unpack_from(dex.data, dex.strings.offset(index))[0]
    length, start = dex.uleb(offset)
    dex.data_range(start, length + 1, "fixture annotation string")
    text = dex.data[start:start + length]
    if dex.data[start + length] != 0 or b"\x00" in text:
        raise ValueError("the fixture annotation string differs from ASCII MUTF-8")
    return text.decode("ascii")


def annotation(dex, offset, depth, offsets):
    type_index, offset = dex.uleb(offset)
    descriptor = dex.descriptor(type_index)
    count, offset = dex.uleb(offset)
    if count > 16:
        raise ValueError("fixture annotation exceeds its element bound")
    values = {}
    previous = -1
    for _ in range(count):
        name_index, offset = dex.uleb(offset)
        if name_index <= previous:
            raise ValueError("fixture annotation names are repeated or out of order")
        previous = name_index
        name = dex.string(name_index)
        if descriptor == DECLARING:
            offsets[name] = offset
        value, offset = encoded_value(dex, offset, depth, offsets)
        values[name] = value
    return {"annotation": descriptor, "values": values}, offset


def encoded_value(dex, offset, depth, offsets):
    dex.data_range(offset, 1, "fixture encoded value")
    header = dex.data[offset]
    tag, arg = header & 0x1f, header >> 5
    offset += 1
    if depth > 6:
        raise ValueError("fixture encoded value exceeds its nesting bound")
    if tag == 0x1f and arg <= 1:
        return bool(arg), offset
    if tag == 0x1d and arg == 0:
        return annotation(dex, offset, depth + 1, offsets)
    if tag == 0x1c and arg == 0:
        count, offset = dex.uleb(offset)
        if count > 16:
            raise ValueError("fixture array exceeds its element bound")
        elements = []
        for _ in range(count):
            value, offset = encoded_value(dex, offset, depth + 1, offsets)
            elements.append(value)
        return elements, offset
    if tag not in (0x04, 0x17, 0x18, 0x1b) or arg > 3:
        raise ValueError(f"unexpected fixture encoded value header 0x{header:02x}")
    end = offset + arg + 1
    dex.data_range(offset, arg + 1, "fixture scalar payload")
    index = int.from_bytes(dex.data[offset:end], "little", signed=(tag == 0x04))
    if tag == 0x04:
        return index, end
    if tag == 0x17:
        return original_string(dex, index), end
    if tag == 0x18:
        return {"class": dex.descriptor(index)}, end
    field = dex.fields.offset(index)
    owner = dex.descriptor(U16.unpack_from(dex.data, field)[0])
    field_type = dex.descriptor(U16.unpack_from(dex.data, field + 2)[0])
    name = dex.string(U32.unpack_from(dex.data, field + 4)[0])
    return {"enum": [owner, name, field_type]}, end


def class_defaults(dex, ordinal, owner, offsets):
    entry = dex.classes.offset(ordinal)
    owner_index, flags = (U32.unpack_from(dex.data, entry + offset)[0] for offset in (0, 4))
    if dex.descriptor(owner_index) != owner or flags & 0x2200 != 0x2200:
        raise ValueError("fixture annotation interface identity or flags have changed")
    directory = U32.unpack_from(dex.data, entry + 20)[0]
    if not directory:
        return {}
    dex.data_range(directory, 16, "fixture annotations directory")
    ann_set = U32.unpack_from(dex.data, directory)[0]
    dex.data_range(ann_set, 8, "fixture annotation set")
    if U32.unpack_from(dex.data, ann_set)[0] != 1:
        raise ValueError("the fixture annotation set should hold one carrier")
    item = U32.unpack_from(dex.data, ann_set + 4)[0]
    dex.data_range(item, 1, "fixture annotation visibility")
    outer, _ = annotation(dex, item + 1, 0, offsets)
    values = outer["values"]
    if dex.data[item] != 2 or outer["annotation"] != CARRIER or set(values) != {"value"}:
        raise ValueError("the system annotation default carrier has changed")
    payload = values["value"]
    if not isinstance(payload, dict) or payload.get("annotation") != owner:
        raise ValueError("the default carrier does not refer to its declaring annotation interface")
    return payload["values"]


def malformed_count(dex, offset):
    if dex.data[offset:offset + 2] != bytes((0x04, 0xdf)):
        raise ValueError("count must have the original one-byte signed int encoding")
    indices = [index for index in range(dex.strings.count) if dex.string(index) == "west"]
    if len(indices) != 1 or indices[0] > 0xff:
        raise ValueError("the replacement string needs one byte in the fixture encoding")
    data = bytearray(dex.data)
    data[offset:offset + 2] = bytes((0x17, indices[0]))
    data[12:32] = sha1(data[32:]).digest()
    U32.pack_into(data, 8, adler32(data[12:]))
    replacement, _ = encoded_value(Dex(bytes(data)), offset, 0, {})
    if replacement != "west":
        raise ValueError("the malformed fixture did not encode the target string")
    return data


def original(path, cases):
    with path.open("rb") as stream:
        data = stream.read(16 * 1024 + 1)
    if len(data) > 16 * 1024:
        raise ValueError("input exceeds the fixture DEX size bound")
    dex = Dex(data)
    expected_members = {}
    for key, case in cases.items():
        owner, name = key.split(".")
        expected_members[case["at"]] = f"Lfixture/{owner};->{name}(){case['return']}"
    if dict(dex.methods()) != expected_members:
        raise ValueError("fixture original method identities have changed")
    observations = {}
    offsets = {}
    for ordinal, owner in enumerate(OWNERS):
        defaults = class_defaults(dex, ordinal, f"Lfixture/{owner};", offsets)
        members = {key.split(".")[1]: case for key, case in cases.items() if key.startswith(owner + ".")}
        if defaults.keys() - members.keys():
            raise ValueError("fixture annotation has default bindings for undeclared names")
        for name, case in members.items():
            observations[f"{owner}.{name}"] = [case["return"], defaults.get(name)]
    return observations, malformed_count(dex, offsets["count"])


def call(argv, directory, timeout):
    result = run(argv, cwd=directory, timeout=timeout, output_limit=65536, diagnostic_limit=65536)
    if result.returncode:
        detail = (result.stderr or result.stdout).decode("utf-8", "replace")
        raise ValueError(detail.replace(str(directory), "<fixture>")[:1500]
                         or f"{Path(argv[0]).name} exited with status {result.returncode}")
    return result.stdout.decode("utf-8", "strict")


def execute(sources, directory, timeout):
    directory.mkdir()
    paths = []
    for owner, source in sources.items():
        path = directory / f"{owner}.java"
        path.write_text(source, encoding="utf-8")
        paths.append(path)
    call([*JAVA, "-m", "jdk.compiler/com.sun.tools.javac.Main", "-proc:none", "-encoding",
          "UTF-8", "-d", directory, *paths, ROOT / "CheckDexDefaults.java"], directory, timeout)
    lines = call([*JAVA, "-cp", directory, "fixture.CheckDexDefaults"], directory, timeout).splitlines()
    observations = {}
    for line in lines:
        member, descriptor, value = line.split("\t")
        if member in observations:
            raise ValueError("the observer returned duplicate annotation members")
        observations[member] = [descriptor, json.loads(value)]
    return observations


def class_source(binary, artifact, ordinal, owner, work, timeout):
    descriptor = f"Lfixture/{owner};"
    output = call([binary, "get-class", artifact, descriptor, "--format", "json"], work, timeout)
    publication = json.loads(output)
    if (publication["schema_version"] != 5 or publication["command"] != "get-class"
            or publication["selection"]["class_at"] != f"e0:l0:c{ordinal}::{descriptor}"
            or publication["issues"]):
        raise ValueError(f"complete class publication for {descriptor} is unavailable")
    source = publication["result"]["text"]
    if not isinstance(source, str) or not source.strip():
        raise ValueError(f"empty class publication for {descriptor}")
    return source


def same(first, second):
    return json.dumps(first, sort_keys=True) == json.dumps(second, sort_keys=True)


def malformed_publication(binary, artifact, command, selector, work, timeout):
    result = run([binary, command, artifact, selector, "--format", "json"], cwd=work,
                 timeout=timeout, output_limit=65536, diagnostic_limit=65536)
    publication = json.loads(result.stdout)
    if (publication["schema_version"] != 5 or publication["command"] != command
            or publication["selection"]["class_at"] != f"e0:l0:c0::{DECLARING}"):
        raise ValueError("unexpected malformed-fixture publication identity")
    source = publication["result"]["text"]
    issues = [{key: issue[key] for key in ("method_id", "class", "name", "prototype", "stage")}
              for issue in publication["issues"]]
    return source, {"exit": result.returncode, "issues": issues}


def malformed_scope(binary, data, raw, cases, work, timeout):
    count, labels = cases["DexDefaults.count"], cases["DexDefaults.labels"]
    issue = {"method_id": int(count["at"].rsplit(":m", 1)[1]), "class": DECLARING,
             "name": "count", "prototype": "()I", "stage": "method metadata"}
    reported = {"exit": 1, "issues": [issue]}
    expected = {"class": reported, "selected_count": reported,
                "selected_labels": {"exit": 0, "issues": []}, "default_mismatches": []}
    row = {"id": "malformed-count-keeps-original-scope", "expected": expected}
    artifact = work / "malformed-count.dex"
    artifact.write_bytes(data)
    try:
        source, whole = malformed_publication(binary, artifact, "get-class", DECLARING, work, timeout)
        _, selected_count = malformed_publication(binary, artifact, "get-method", count["at"], work, timeout)
        labels_source, selected_labels = malformed_publication(binary, artifact, "get-method",
                                                               labels["at"], work, timeout)
        label = class_source(binary, artifact, 1, "DexLabel", work, timeout)
        expected_defaults = {**raw, "DexDefaults.count": ["I", None]}
        mismatches = []
        for name, annotation in (("class", source), ("labels", labels_source)):
            values = execute({"DexDefaults": annotation, "DexLabel": label}, work / name, timeout)
            mismatches.extend(f"{name}:{key}" for key in expected_defaults.keys() | values.keys()
                              if not same(values.get(key), expected_defaults.get(key)))
        observed = {"class": whole, "selected_count": selected_count,
                    "selected_labels": selected_labels, "default_mismatches": sorted(mismatches)}
        row.update(observed=observed, status="PASS" if same(observed, expected) else "FAIL")
    except (OSError, ValueError, KeyError, SubprocessError) as failure:
        row.update(status="UNCHECKED", detail=str(failure))
    return row


def probe(binary, artifact, timeout):
    binary, artifact = Path(binary).resolve(), Path(artifact).resolve()
    cases = json.loads((ROOT / "scenarios.json").read_text(encoding="utf-8"))
    raw, malformed = original(artifact, cases)
    recovered, error = {}, None
    with TemporaryDirectory(prefix="annotation-default-dex-") as path:
        work = Path(path)
        sources = {owner: (ROOT / f"{owner}.java").read_text(encoding="utf-8") for owner in OWNERS}
        authored = execute(sources, work / "authored", timeout)
        try:
            publications = {owner: class_source(binary, artifact, ordinal, owner, work, timeout)
                            for ordinal, owner in enumerate(OWNERS)}
            recovered = execute(publications, work / "recovered", timeout)
        except (OSError, ValueError, KeyError, SubprocessError) as failure:
            error = str(failure)
        scope = malformed_scope(binary, malformed, raw, cases, work, timeout)
    results = {}
    for member, case in cases.items():
        expected = [case["return"], case["value"]]
        observed = recovered.get(member)
        row = {"id": member, "expected": expected, "observed": observed,
               "source_control": authored.get(member), "original_dex": raw[member]}
        if not same(raw[member], expected) or not same(authored.get(member), expected):
            row.update(status="SETUP_ERROR", detail="original DEX or authored Java disagrees with its expectation")
        elif error:
            row.update(status="UNCHECKED", detail=error)
        else:
            row["status"] = "PASS" if same(observed, expected) else "FAIL"
        results[case["at"]] = [row]
    results[cases["DexDefaults.count"]["at"]].append(scope)
    return results
