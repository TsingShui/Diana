"""Check preparation and static reads against authored initialization scenarios."""

from hashlib import sha256
import json
from pathlib import Path
from subprocess import SubprocessError
from tempfile import TemporaryDirectory
from zipfile import ZipFile, ZipInfo

from corpus_models import diagnostics, invoke
from dex_calls import method_ref
from dex_truth import Dex, U32
from dex_witness import program, register_words, static_field


ROOT = Path(__file__).resolve().parent
DEX_SHA = "6858633ee853a7820af1ffe331ef31776fc73ca920e9cbe138f860893895ab36"
JAVA = ["java", "-XX:+IgnoreUnrecognizedVMOptions", "-XX:-UsePerfData", "-Xmx128m"]
CLASSES = {"PreparationParent": 1, "PreparedDefaults": 2, "ReadPreparedDefaults": 3}
OWNER = "Lfixture/PreparedDefaults;"
HOOKS = "Lfixture/PreparationEvents;"
READER = "Lfixture/ReadPreparedDefaults;"
SIGNATURES = {
    "e0:l0:c1:m2": "Lfixture/PreparationParent;-><clinit>()V",
    "e0:l0:c3:m3": f"{READER}->encodedZero()I",
    "e0:l0:c3:m4": f"{READER}->preparedNull()Ljava/lang/String;",
    "e0:l0:c3:m5": f"{READER}->preparedZero()I",
}
SCENARIO_METHOD = {
    "control": "e0:l0:c3:m3",
    "success-null": "e0:l0:c3:m4",
    "fault-null": "e0:l0:c3:m4",
    "success-zero": "e0:l0:c3:m5",
    "fault-zero": "e0:l0:c3:m5",
}
EXECUTION_ERRORS = (OSError, ValueError, KeyError, SubprocessError)


def field_preparation(dex):
    definition = dex.classes.offset(CLASSES["PreparedDefaults"])
    owner = dex.descriptor(U32.unpack_from(dex.data, definition)[0])
    parent = dex.descriptor(U32.unpack_from(dex.data, definition + 8)[0])
    data, values = (U32.unpack_from(dex.data, definition + offset)[0] for offset in (24, 28))
    if owner != OWNER or parent != "Lfixture/PreparationParent;" or not data or not values:
        raise ValueError("original preparation owner, parent or tables differ")
    counts = []
    for _ in range(4):
        count, data = dex.uleb(data)
        counts.append(count)
    if counts != [4, 0, 0, 0]:
        raise ValueError("original child must have four static fields and no method bodies")
    array_size, cursor = dex.uleb(values)
    if array_size != 2:
        raise ValueError("exactly two preparations must be encoded")
    expected = [
        ("A_ENCODED_ZERO", "I", 0),
        ("B_ENCODED_SENTINEL", "I", 9),
        ("C_PREPARED_ZERO", "I", None),
        ("D_PREPARED_NULL", "Ljava/lang/String;", None),
    ]
    index = 0
    fields = []
    for ordinal, (name, descriptor, expected_value) in enumerate(expected):
        delta, data = dex.uleb(data)
        flags, data = dex.uleb(data)
        index += delta
        if static_field(dex, index) != (OWNER, name, descriptor) or flags & 0x18 != 0x18:
            raise ValueError("original static field or ordering differs")
        encoded = ordinal < array_size
        value = None
        if encoded:
            dex.data_range(cursor, 2, "fixture integer value")
            if dex.data[cursor] != 4:
                raise ValueError("fixture requires a signed one-byte encoded integer")
            value = dex.data[cursor + 1]
            cursor += 2
        if value != expected_value:
            raise ValueError("original field preparation disagrees with the authored value")
        fields.append({"member": name, "descriptor": descriptor, "encoded": encoded, "value": value})
    return {"parent": parent, "child_has_initializer": False, "fields": fields}


def original_facts(artifact, directory):
    with artifact.open("rb") as stream:
        data = stream.read(16 * 1024 + 1)
    if len(data) > 16 * 1024 or sha256(data).hexdigest() != DEX_SHA:
        raise ValueError("input is not the authored prepared-default DEX")
    raw = field_preparation(Dex(data))
    archive = directory / "original.zip"
    with ZipFile(archive, "x") as wrapped:
        wrapped.writestr(ZipInfo("classes.dex", (1980, 1, 1, 0, 0, 0)), data)
    checksum = sha256(archive.read_bytes()).hexdigest()
    expected_words = {
        "e0:l0:c1:m2": [0x71, 0x71, 0x0e],
        "e0:l0:c3:m3": [0x12, 0x0f],
        "e0:l0:c3:m4": [0x62, 0x11],
        "e0:l0:c3:m5": [0x60, 0x0f],
    }
    reads = {
        "e0:l0:c3:m4": (OWNER, "D_PREPARED_NULL", "Ljava/lang/String;"),
        "e0:l0:c3:m5": (OWNER, "C_PREPARED_ZERO", "I"),
    }
    for at, member in SIGNATURES.items():
        decoded = program(archive, checksum, at, member)
        instructions = list(decoded.instructions.values())
        if [node.word for node in instructions] != expected_words[at] or decoded.protection:
            raise ValueError(f"unexpected original instructions for {member}")
        if at in reads and static_field(decoded.dex, instructions[0].operands[0]) != reads[at]:
            raise ValueError(f"unexpected original field reference for {member}")
        if at == "e0:l0:c1:m2":
            calls = [(method_ref(decoded.dex, node.operands[0]), register_words(node))
                     for node in instructions[:2]]
            if calls != [((HOOKS, "start", "()V"), ()), ((HOOKS, "finish", "()V"), ())]:
                raise ValueError("original superclass initialization order differs")
    raw["original_reads"] = reads
    raw["parent_calls"] = ["start", "finish"]
    return raw


def call(args, directory, timeout):
    result = invoke(args, directory, timeout)
    if result.returncode:
        detail = diagnostics(result.stderr, directory)
        raise ValueError(detail or f"{Path(args[0]).name} exited with status {result.returncode}")
    return result.stdout.decode("utf-8")


def execute(binary, artifact, directory, scenarios, timeout):
    directory.mkdir()
    sources = [ROOT / "PreparationEvents.java", ROOT / "CheckPreparedDefaults.java"]
    for name, index in CLASSES.items():
        if binary is None:
            sources.append(ROOT / f"{name}.java")
            continue
        locator = f"e0:l0:c{index}"
        body = call([str(binary), "get-class", str(artifact), locator, "--format", "json"],
                    directory, timeout)
        publication = json.loads(body)
        expected = f"{locator}::Lfixture/{name};"
        if (publication["schema_version"] != 5 or publication["command"] != "get-class"
                or publication["selection"]["class_at"] != expected or publication["issues"]):
            raise ValueError(f"incomplete class publication for {expected}")
        source = directory / f"{name}.java"
        source.write_text(publication["result"]["text"], encoding="utf-8")
        sources.append(source)
    output = directory / "classes"
    call([*JAVA, "-m", "jdk.compiler/com.sun.tools.javac.Main", "-proc:none",
          "-encoding", "UTF-8", "-d", str(output), *map(str, sources)], directory, timeout)
    observations = {}
    for name in scenarios:
        trace = call([*JAVA, "-cp", str(output), "fixture.CheckPreparedDefaults", name],
                     directory, timeout).splitlines()
        if len(trace) != 1:
            raise ValueError(f"expected one event trace for {name}")
        observations[name] = trace[0].split(",")
    return observations


def probe(binary, artifact, timeout):
    binary, artifact = Path(binary).resolve(), Path(artifact).resolve()
    expected = json.loads((ROOT / "scenarios.json").read_text(encoding="utf-8"))
    if set(expected) != set(SCENARIO_METHOD):
        raise ValueError("scenario expectation set disagrees with the fixture")
    raw, control, recovered = None, {}, {}
    setup_error = candidate_error = None
    with TemporaryDirectory(prefix="prepared-static-final-") as tmp:
        directory = Path(tmp)
        try:
            raw = original_facts(artifact, directory)
            control = execute(None, artifact, directory / "authored", expected, timeout)
        except EXECUTION_ERRORS as error:
            setup_error = str(error)
        if setup_error is None:
            try:
                recovered = execute(binary, artifact, directory / "recovered", expected, timeout)
            except EXECUTION_ERRORS as error:
                candidate_error = str(error)
    results = {}
    for name, wanted in expected.items():
        observed = recovered.get(name)
        row = {"id": name, "expected": wanted, "observed": observed,
               "source_control": control.get(name), "original_dex": raw}
        if setup_error is not None or control.get(name) != wanted:
            row.update(status="SETUP_ERROR",
                       detail=setup_error or "authored Java trace disagrees with the expectation")
        elif candidate_error is not None:
            row.update(status="UNCHECKED", detail=candidate_error)
        else:
            row["status"] = "PASS" if observed == wanted else "FAIL"
        results.setdefault(SCENARIO_METHOD[name], []).append(row)
    return results
