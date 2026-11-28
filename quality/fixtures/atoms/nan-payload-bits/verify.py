"""Observe quiet NaN payloads from raw DEX and the exact class publication."""

from hashlib import sha256
import json
from pathlib import Path
from subprocess import SubprocessError
from tempfile import TemporaryDirectory

from dex_calls import method_code, method_ref
from dex_truth import Dex, U16, U32
from dex_witness import decode_instructions, exception_table, register_words, static_definition, static_field
from tool_capture import run


ROOT = Path(__file__).resolve().parent
HASH = "6728778c3fa2898342a61d0fdf55cdee697670d25afa761c53d446c92cf80614"
OWNER = "Lfixture/ScalarNan;"
FAULT = "Ljava/lang/ArithmeticException;"
METHODS = {
    "bits": (0, "(Z)I", 1),
    "canonicalFloat": (1, "()F", 0),
    "doubleValue": (2, "()D", 0),
    "floatValue": (3, "()F", 0),
    "inspectDouble": (4, f"(D{FAULT})J", 3),
    "protectedBits": (5, f"(Z{FAULT})J", 2),
    "store": (6, "()V", 0),
}
JAVA = ["java", "-XX:+IgnoreUnrecognizedVMOptions", "-XX:-UsePerfData", "-Xmx128m"]
EXECUTION_ERRORS = (OSError, ValueError, KeyError, SubprocessError)


def read_methods(dex):
    methods = {}
    for name, (index, proto, args) in METHODS.items():
        code = method_code(dex, 0, index, f"{OWNER}->{name}{proto}")
        if not static_definition(dex, 0, index):
            raise ValueError(f"{name} is not static")
        dex.data_range(code, 16, "fixture code item header")
        registers, inputs, tries = (U16.unpack_from(dex.data, code + offset)[0]
                                    for offset in (0, 2, 6))
        units = U32.unpack_from(dex.data, code + 12)[0]
        if inputs != args or not inputs <= registers <= 8 or not 0 < units <= 64:
            raise ValueError(f"{name} exceeds the fixture register or code bound")
        start = code + 16
        dex.data_range(start, 2 * units, "fixture instructions")
        decoded = decode_instructions(dex, start, units)
        protection = exception_table(dex, start + 2 * units, units, tries, decoded)
        if bool(protection) != (name == "protectedBits"):
            raise ValueError(f"unexpected catch coverage for {name}")
        methods[name] = (registers - inputs, decoded, protection)
    return methods


def literal(node):
    if node.word == 0x15:
        return node.operands[0] << 16
    if node.word in (0x14, 0x18):
        return sum(piece << (16 * offset) for offset, piece in enumerate(node.operands))
    raise ValueError("selected fixture literal is not a raw constant into v0")


def invoke(dex, node, binding, registers):
    if (node.opcode != 0x71 or method_ref(dex, node.operands[0]) != binding
            or register_words(node) != registers):
        raise ValueError(f"unexpected argument words or original method binding for {binding}")


def field(dex, node, word, name, descriptor):
    if node.word != word or static_field(dex, node.operands[0]) != (OWNER, name, descriptor):
        raise ValueError(f"unexpected original field operation on {name}:{descriptor}")


def helper_facts(dex, selected):
    base, code, _ = selected
    operations = list(code.values())
    if [node.opcode for node in operations] != [0x71, 0x0b, 0x68, 0x38, 0x27, 0x10]:
        raise ValueError("unexpected inspectDouble helper structure")
    call, moved, store, test, throw, returned = operations
    fault = base + 2
    invoke(dex, call, ("Ljava/lang/Double;", "doubleToRawLongBits", "(D)J"), (base, base + 1))
    field(dex, store, 0x68, "lastBits", "J")
    if (test.word >> 8 != fault or test.target != returned.pc or throw.word >> 8 != fault
            or moved.word >> 8 != 0 or returned.word >> 8 != 0):
        raise ValueError("the helper changes the original exception or returned bits")
    return {"raw_call_pc": call.pc, "write_pc": store.pc, "fault_pc": throw.pc}


def original_observation(dex, methods, helper, method, choice, fault):
    base, code, protection = methods[method]
    initial = code[0]
    if method in ("bits", "protectedBits"):
        if initial.opcode != 0x38 or initial.word >> 8 != base:
            raise ValueError("selection does not test its original Boolean argument")
        selected = code[initial.next if choice else initial.target]
    else:
        selected = initial
    raw = literal(selected)
    narrow = method in ("floatValue", "canonicalFloat", "bits")
    exponent, quiet = (0x7f800000, 0x00400000) if narrow else (0x7ff0000000000000, 0x0008000000000000)
    if raw & exponent != exponent or not raw & quiet:
        raise ValueError("the fixture selected a value outside its quiet-NaN model")
    formatted = f"{raw:0{8 if narrow else 16}x}"
    consumer = code[selected.next]
    if consumer.opcode == 0x28:
        consumer = code[consumer.target]
    evidence = {"literal_pc": selected.pc, "consumer_pc": consumer.pc, "bits": formatted}
    if method in ("floatValue", "doubleValue", "canonicalFloat"):
        if consumer.word != (0x0f if narrow else 0x10):
            raise ValueError("the selected return changes the original literal")
    elif method == "store":
        field(dex, consumer, 0x68, "stored", "D")
        if code[consumer.next].word != 0x0e:
            raise ValueError("unexpected effect after the original runtime store")
        evidence["field"] = "stored:D"
    elif method == "bits":
        invoke(dex, consumer, ("Ljava/lang/Float;", "floatToRawIntBits", "(F)I"), (0,))
        moved = code[consumer.next]
        if moved.word != 0x0a or code[moved.next].word != 0x0f:
            raise ValueError("the original raw-bit call does not feed a direct return")
    else:
        invoke(dex, consumer, (OWNER, "inspectDouble", f"(D{FAULT})J"), (0, 1, base + 1))
        moved = code[consumer.next]
        if moved.word != 0x0b or code[moved.next].word != 0x10:
            raise ValueError("the helper's normal result is not returned")
        if len(protection) != 1 or len(protection[0].clauses) != 1:
            raise ValueError("expected one original typed catch")
        region = protection[0]
        descriptor, address = region.clauses[0]
        if (descriptor != FAULT or region.start != consumer.pc or region.end != moved.next):
            raise ValueError("the original typed catch covers a different helper boundary")
        captured = code[address]
        if captured.opcode != 0x0d:
            raise ValueError("the handler does not capture the original exception")
        put = code[captured.next]
        get = code[put.next]
        field(dex, put, (captured.word & 0xff00) | 0x69, "caught", FAULT)
        field(dex, get, 0x61, "lastBits", "J")
        if code[get.next].word != 0x10:
            raise ValueError("the caught path does not return the recorded helper bits")
        evidence.update(helper, catch_pc=address, catch_type=descriptor, outcome="catch" if fault else "return")
        formatted += f":{formatted}:" + ("same-fault" if fault else "normal")
    return {"value": formatted, "trace": evidence}


def original(artifact, scenarios):
    with artifact.open("rb") as stream:
        data = stream.read(16 * 1024 + 1)
    if len(data) > 16 * 1024 or sha256(data).hexdigest() != HASH:
        raise ValueError("input differs from the authored quiet-NaN DEX fixture")
    dex = Dex(data)
    methods = read_methods(dex)
    helper = helper_facts(dex, methods["inspectDouble"])
    return {case: original_observation(dex, methods, helper, method, choice, fault)
            for case, method, choice, fault, _ in scenarios}


def call(argv, directory, timeout):
    result = run(argv, cwd=directory, timeout=timeout, output_limit=65536, diagnostic_limit=65536)
    if result.returncode:
        detail = (result.stderr or result.stdout).decode("utf-8", "replace")
        raise ValueError(detail.replace(str(directory), "<fixture>")[:1500]
                         or f"{Path(argv[0]).name} exited with status {result.returncode}")
    return result.stdout.decode("utf-8", "strict")


def execute(source, directory, scenarios, timeout):
    directory.mkdir()
    java = directory / "ScalarNan.java"
    java.write_text(source, encoding="utf-8")
    call([*JAVA, "-m", "jdk.compiler/com.sun.tools.javac.Main", "-proc:none", "-encoding",
          "UTF-8", "-d", directory, java, ROOT / "CheckScalars.java"], directory, timeout)
    values = {}
    for case, method, choice, fault, _ in scenarios:
        result = call([*JAVA, "-cp", directory, "fixture.CheckScalars", method,
                       str(choice).lower(), str(fault).lower()], directory, timeout).splitlines()
        if len(result) != 1:
            raise ValueError("observer must publish exactly one value")
        values[case] = result[0]
    return values


def class_source(binary, artifact, scratch, timeout):
    text = call([binary, "get-class", artifact, OWNER, "--format", "json"], scratch, timeout)
    publication = json.loads(text)
    if (publication["schema_version"] != 5 or publication["command"] != "get-class"
            or publication["selection"]["class_at"] != f"e0:l0:c0::{OWNER}" or publication["issues"]):
        raise ValueError("complete ScalarNan class publication unavailable")
    source = publication["result"]["text"]
    if not isinstance(source, str) or not source.strip():
        raise ValueError("ScalarNan publication is empty")
    return source


def probe(binary, artifact, timeout):
    binary, artifact = Path(binary).resolve(), Path(artifact).resolve()
    scenarios = json.loads((ROOT / "scenarios.json").read_text(encoding="utf-8"))
    ids = {case for case, _, _, _, _ in scenarios}
    methods = {method for _, method, _, _, _ in scenarios}
    if len(ids) != len(scenarios) or methods != METHODS.keys() - {"inspectDouble"}:
        raise ValueError("scenario selection differs from the authored quiet-NaN fixture")
    raw = original(artifact, scenarios)
    recovered, error = {}, None
    with TemporaryDirectory(prefix="nan-payload-dex-") as path:
        work = Path(path)
        source = (ROOT / "ScalarNan.java").read_text(encoding="utf-8")
        authored = execute(source, work / "authored", scenarios, timeout)
        try:
            candidate = class_source(binary, artifact, work, timeout)
            recovered = execute(candidate, work / "recovered", scenarios, timeout)
        except EXECUTION_ERRORS as failure:
            error = str(failure)
    results = {}
    for case, method, choice, fault, expected in scenarios:
        observed = recovered.get(case)
        row = {"id": case, "arguments": [choice, fault], "expected": expected, "observed": observed,
               "source_control": authored[case], "original_dex": raw[case]}
        if authored[case] != expected or raw[case]["value"] != expected:
            row.update(status="SETUP_ERROR", detail="original DEX or authored Java disagrees with the scenario")
        elif observed is None:
            row.update(status="UNCHECKED", detail=error)
        else:
            row["status"] = "PASS" if observed == expected else "FAIL"
        results.setdefault(f"e0:l0:c0:m{METHODS[method][0]}", []).append(row)
    return results
