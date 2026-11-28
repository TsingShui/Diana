"""Verify foreign static writes between classes with identical simple names."""

import hashlib
import json
from pathlib import Path
from subprocess import SubprocessError
from tempfile import TemporaryDirectory
from zipfile import ZipFile, ZipInfo

from corpus_models import diagnostics, invoke
from dex_calls import method_ref
from dex_witness import program, register_words, signed, static_field


ROOT = Path(__file__).resolve().parent
DEX_SHA = "2231dc8ac6e9c5eb6504cce61cf15b44068047c9824fffffba34e87216ab3d1b"
JAVA = ["java", "-XX:+IgnoreUnrecognizedVMOptions", "-XX:-UsePerfData", "-Xmx128m"]
CLASSES = {"origin/LateOwner": 1, "origin/PrefixOwner": 2,
           "outside/LateOwner": 3, "outside/PrefixOwner": 4}
CASES = {
    "prefix": ("PrefixOwner", 3, 4, 7, ["own:0", "foreign:7"]),
    "late": ("LateOwner", 1, 2, 5, ["begin:1:0", "own:0", "foreign:5"]),
}
EXECUTION_ERRORS = (OSError, ValueError, KeyError, SubprocessError)


def call(args, directory, timeout):
    result = invoke(args, directory, timeout)
    if result.returncode:
        detail = diagnostics(result.stderr, directory)
        raise ValueError(detail or f"{Path(args[0]).name} exited with status {result.returncode}")
    return result.stdout.decode("utf-8")


def method(name, index):
    return f"e0:l0:c{CLASSES[f'origin/{name}']}:m{index}"


def original_instructions(artifact, directory):
    with Path(artifact).open("rb") as original:
        data = original.read(64 * 1024 + 1)
    if len(data) > 64 * 1024 or hashlib.sha256(data).hexdigest() != DEX_SHA:
        raise ValueError("input differs from the independently authored ownership fixture")
    archive = directory / "original.zip"
    with ZipFile(archive, "w") as wrapped:
        wrapped.writestr(ZipInfo("classes.dex", (1980, 1, 1, 0, 0, 0)), data)
    checksum = hashlib.sha256(archive.read_bytes()).hexdigest()
    cases = {}
    for key, (name, initializer_index, observer_index, value, _) in CASES.items():
        origin = f"Lorigin/{name};"
        outside = f"Loutside/{name};"
        initial_at = method(name, initializer_index)
        observed_at = method(name, observer_index)
        initializer = program(archive, checksum, initial_at, f"{origin}-><clinit>()V")
        observer = program(archive, checksum, observed_at, f"{origin}->observed()I")
        init_trace = []
        read_trace = []
        for disassembly, trace in ((initializer, init_trace), (observer, read_trace)):
            for pc, insn in sorted(disassembly.instructions.items()):
                op = insn.opcode
                register = insn.word >> 8
                if op == 0x12:
                    operands = ["const/4", register & 0xf, signed(register >> 4, 4)]
                elif op in (0x60, 0x67):
                    operands = ["sget" if op == 0x60 else "sput", register,
                                *static_field(disassembly.dex, insn.operands[0])]
                elif op == 0x71:
                    operands = ["invoke-static", register_words(insn),
                                *method_ref(disassembly.dex, insn.operands[0])]
                elif op == 0x0e:
                    operands = ["return-void"]
                elif op == 0x0f:
                    operands = ["return", register]
                else:
                    raise ValueError(f"unsupported ownership fixture opcode {op:02x}")
                trace.append({"pc": pc, "operands": operands})
        init_expected = ([["invoke-static", (), "Lcontrol/OwnerEvents;", "begin", "()V"]]
                         if key == "late" else [])
        init_expected += [["const/4", 0, value],
                          ["sput", 0, outside, "value", "I"], ["return-void"]]
        if ([row["operands"] for row in init_trace] != init_expected
                or [row["operands"] for row in read_trace] !=
                [["sget", 0, origin, "value", "I"], ["return", 0]]
                or initializer.protection or observer.protection):
            raise ValueError(f"unexpected original ownership/transfer at {initial_at}")
        cases[key] = {"initializer_at": initial_at, "initializer": init_trace,
                      "observer_at": observed_at, "observer": read_trace}
    return cases


def execute(binary, artifact, location, timeout):
    location.mkdir()
    sources = sorted((ROOT / "original" / "control").glob("*.java"))
    for owner, index in CLASSES.items():
        if binary is None:
            sources.append(ROOT / "original" / f"{owner}.java")
            continue
        locator = f"e0:l0:c{index}"
        response = call([str(binary), "get-class", str(artifact), locator,
                         "--format", "json"], location, timeout)
        publication = json.loads(response)
        expected = f"{locator}::L{owner};"
        if (publication["schema_version"] != 5 or publication["command"] != "get-class"
                or publication["selection"]["class_at"] != expected or publication["issues"] != []):
            raise ValueError(f"publication unavailable for {expected}")
        text = publication["result"]["text"]
        if not isinstance(text, str) or not text.strip():
            raise ValueError(f"publication missing for {expected}")
        file = location / f"{owner}.java"
        file.parent.mkdir(exist_ok=True)
        file.write_text(text, encoding="utf-8")
        sources.append(file)
    compiled = location / "classes"
    call([*JAVA, "-m", "jdk.compiler/com.sun.tools.javac.Main", "-proc:none", "-encoding",
          "UTF-8", "-d", compiled, *sources], location, timeout)
    result = {}
    for case in CASES:
        text = call([*JAVA, "-cp", compiled, "control.ObserveOwners", case], location, timeout)
        lines = text.splitlines()
        if len(lines) != 1:
            raise ValueError(f"expected a single event trace for {case}")
        result[case] = lines[0].split(",")
    return result


def probe(binary, artifact, timeout):
    binary, artifact = Path(binary).resolve(), Path(artifact).resolve()
    setup_error = candidate_error = None
    raw, reference, recovered = {}, {}, {}
    with TemporaryDirectory(prefix="blank-final-owners-") as tmp:
        directory = Path(tmp)
        try:
            raw = original_instructions(artifact, directory)
            reference = execute(None, artifact, directory / "authored", timeout)
        except EXECUTION_ERRORS as error:
            setup_error = str(error)
        if setup_error is None:
            try:
                recovered = execute(binary, artifact, directory / "recovered", timeout)
            except EXECUTION_ERRORS as error:
                candidate_error = str(error)
    results = {}
    for case, (name, _, observed_index, _, wanted) in CASES.items():
        row = {"id": case, "expected": wanted, "observed": recovered.get(case),
               "source_control": reference.get(case), "original_dex": raw.get(case)}
        if setup_error is not None or row["source_control"] != wanted:
            row.update(status="SETUP_ERROR",
                       detail=setup_error or "authored Java control disagrees with expectation")
        elif candidate_error is not None:
            row.update(status="UNCHECKED", detail=candidate_error)
        else:
            row["status"] = "PASS" if row["observed"] == wanted else "FAIL"
        results[method(name, observed_index)] = [row]
    return results
