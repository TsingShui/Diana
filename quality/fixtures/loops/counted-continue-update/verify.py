"""Compare authored and published Java with a bounded replay of fixture DEX."""

import json
from pathlib import Path
from subprocess import SubprocessError
from tempfile import TemporaryDirectory

from dex_calls import method_code, method_ref
from dex_truth import Dex, U16, U32
from dex_witness import COMPARE, COMPARISONS, decode_instructions, register_words, signed, static_field
from tool_capture import run


ROOT = Path(__file__).resolve().parent
OWNER = "Lfixture/CountedContinueUpdate;"
FAULT = "Ljava/lang/ArithmeticException;"
SUBJECTS = {
    "advanceThenStop": (0, f"([II{FAULT})I", 3),
    "holdOnZero": (1, "([I)I", 1),
    "observeAfterAdvance": (2, f"([I{FAULT})I", 2),
    "record": (3, f"(II{FAULT})V", 3),
}
JAVA = ["java", "-XX:+IgnoreUnrecognizedVMOptions", "-XX:-UsePerfData", "-Xmx128m"]


class NullArrayAccess(Exception):
    pass


class SuppliedFault(Exception):
    pass


def original_programs(path):
    with path.open("rb") as stream:
        data = stream.read(16 * 1024 + 1)
    if len(data) > 16 * 1024:
        raise ValueError("input exceeds the fixture DEX size bound")
    dex = Dex(data)
    programs = {}
    for method, (index, prototype, arguments) in SUBJECTS.items():
        offset = method_code(dex, 0, index, f"{OWNER}->{method}{prototype}")
        dex.data_range(offset, 16, "loop fixture code item header")
        registers, inputs, tries = (U16.unpack_from(data, offset + position)[0] for position in (0, 2, 6))
        units = U32.unpack_from(data, offset + 12)[0]
        if inputs != arguments or not inputs <= registers <= 8 or tries or not 0 < units <= 64:
            raise ValueError("unexpected register, catch or code shape in the loop fixture")
        start = offset + 16
        dex.data_range(start, units * 2, "loop fixture instructions")
        programs[method] = registers - inputs, decode_instructions(dex, start, units)
    return dex, programs


def int32(value):
    return signed(value & 0xffffffff, 32)


def branch(node, registers):
    if node.opcode == 0x28:
        return node.target
    if node.opcode < 0x38:
        operands = registers[node.word >> 8 & 15], registers[node.word >> 12]
    else:
        value = registers[node.word >> 8]
        operands = (int(value is not None) if type(value) is not int else value), 0
    if COMPARE[COMPARISONS[node.opcode]](*operands):
        return node.target
    return node.next


def execute_dex(dex, programs, method, arguments, fields, trace):
    locals_count, code = programs[method]
    registers = [None] * locals_count + list(arguments)
    pc = 0
    for _ in range(256):
        node = code[pc]
        word, opcode, next_pc = node.word, node.opcode, node.next
        if opcode == 0x12:
            registers[word >> 8 & 15] = signed(word >> 12, 4)
        elif opcode == 0x21:
            array = registers[word >> 12]
            if array is None:
                raise NullArrayAccess()
            registers[word >> 8 & 15] = len(array)
        elif opcode in (0x44, 0x4b):
            packed = node.operands[0]
            array, index = registers[packed & 255], registers[packed >> 8]
            if type(array) is not list or not 0 <= index < len(array):
                raise ValueError("fixture array reference or index disagrees with the original guard")
            if opcode == 0x44:
                registers[word >> 8] = array[index]
            else:
                array[index] = registers[word >> 8]
                trace.append([method, pc, "array-write", index, array[index]])
        elif opcode in COMPARISONS or opcode == 0x28:
            next_pc = branch(node, registers)
        elif opcode in (0x60, 0x67):
            owner, key, ty = static_field(dex, node.operands[0])
            if owner != OWNER or key not in fields or ty != "I":
                raise ValueError("fixture static field binding changed")
            if opcode == 0x60:
                registers[word >> 8] = fields[key]
            else:
                fields[key] = registers[word >> 8]
                trace.append([method, pc, "static-write", key, fields[key]])
        elif opcode == 0x71:
            target = method_ref(dex, node.operands[0])
            if method == "record" or target != (OWNER, "record", SUBJECTS["record"][1]):
                raise ValueError("fixture helper call binding changed")
            operands = [registers[index] for index in register_words(node)]
            if operands[2] == 0:
                operands[2] = None
            trace.append([method, pc, "record", operands[0], operands[1], operands[2] is not None])
            execute_dex(dex, programs, "record", operands, fields, trace)
        elif opcode in (0xd8, 0xda):
            packed = node.operands[0]
            source, literal = registers[packed & 255], signed(packed >> 8, 8)
            result = source + literal if opcode == 0xd8 else source * literal
            registers[word >> 8] = int32(result)
            if method != "record":
                trace.append([method, pc, "advance", result])
        elif opcode == 0xb0:
            dest, source = word >> 8 & 15, word >> 12
            registers[dest] = int32(registers[dest] + registers[source])
        elif opcode == 0x27:
            value = registers[word >> 8]
            if not isinstance(value, SuppliedFault):
                raise ValueError("original throw does not carry the supplied exception")
            trace.append([method, pc, "throw-supplied"])
            raise value
        elif opcode in (0x0e, 0x0f):
            return None if opcode == 0x0e else registers[word >> 8]
        else:
            raise ValueError(f"unmodeled loop fixture opcode {opcode:#x} at {pc}")
        pc = next_pc
    raise ValueError("loop fixture exceeded its DEX step bound")


def original_observation(dex, programs, scenario):
    method = scenario["method"]
    array = None if scenario["values"] is None else list(scenario["values"])
    fault = SuppliedFault() if scenario["fault"] else None
    arguments = {
        "advanceThenStop": (array, scenario["limit"], fault),
        "holdOnZero": (array,),
        "observeAfterAdvance": (array, fault),
    }[method]
    fields = {"state": 17, "events": 3, "lastValue": -99}
    trace = []
    try:
        result = execute_dex(dex, programs, method, arguments, fields, trace)
    except NullArrayAccess:
        outcome = "null-array-fault"
    except SuppliedFault as failure:
        outcome = "supplied-fault" if failure is fault else "different-fault"
    else:
        outcome = f"return:{result}"
    return [outcome, fields["state"], fields["events"], fields["lastValue"], array], trace


def call(argv, directory, timeout):
    capture = run(argv, cwd=directory, timeout=timeout, output_limit=65536, diagnostic_limit=65536)
    if capture.returncode:
        detail = (capture.stderr or capture.stdout).decode("utf-8", "replace")
        raise ValueError(detail.replace(str(directory), "<fixture>")[:1500]
                         or f"{Path(argv[0]).name} exited with status {capture.returncode}")
    return capture.stdout.decode("utf-8", "strict")


def class_source(binary, artifact, work, timeout):
    data = json.loads(call([binary, "get-class", artifact, OWNER, "--format", "json"], work, timeout))
    if (data["schema_version"] != 5 or data["command"] != "get-class" or data["issues"]
            or data["selection"]["class_at"] != f"e0:l0:c0::{OWNER}"):
        raise ValueError("complete loop class publication unavailable")
    source = data["result"]["text"]
    if not isinstance(source, str) or not source.strip():
        raise ValueError("loop class publication is empty")
    return source


def observe(source, work, scenarios, timeout):
    work.mkdir()
    path = work / "CountedContinueUpdate.java"
    path.write_text(source, encoding="utf-8")
    call([*JAVA, "-m", "jdk.compiler/com.sun.tools.javac.Main", "-proc:none", "-encoding", "UTF-8",
          "-d", work, path, ROOT / "CheckCountedContinue.java"], work, timeout)
    result = {}
    for scenario in scenarios:
        values = scenario["values"]
        encoded = "null" if values is None else ",".join(map(str, values))
        text = call([*JAVA, "-cp", work, "fixture.CheckCountedContinue", scenario["method"], encoded,
                     str(scenario["limit"]), str(scenario["fault"]).lower()], work, timeout)
        result[scenario["id"]] = json.loads(text)
    return result


def probe(binary, artifact, timeout):
    binary, artifact = Path(binary).resolve(), Path(artifact).resolve()
    scenarios = json.loads((ROOT / "scenarios.json").read_text(encoding="utf-8"))
    dex, programs = original_programs(artifact)
    raw = {row["id"]: original_observation(dex, programs, row) for row in scenarios}
    recovered, error = {}, None
    with TemporaryDirectory(prefix="counted-loop-continue-") as path:
        work = Path(path)
        authored = observe((ROOT / "CountedContinueUpdate.java").read_text(encoding="utf-8"),
                            work / "authored", scenarios, timeout)
        try:
            recovered = observe(class_source(binary, artifact, work, timeout), work / "recovered",
                                scenarios, timeout)
        except (OSError, ValueError, KeyError, SubprocessError) as failure:
            error = str(failure)
    result = {}
    for scenario in scenarios:
        name, expected = scenario["id"], scenario["expected"]
        raw_value, trace = raw[name]
        observed = recovered.get(name)
        row = {"id": name, "expected": expected, "observed": observed, "source_control": authored[name],
               "original_dex": raw_value, "trace": trace}
        if raw_value != expected or authored[name] != expected:
            row.update(status="SETUP_ERROR", detail="authored Java or original DEX disagrees with the scenario")
        elif observed is None:
            row.update(status="UNCHECKED", detail=error)
        else:
            row["status"] = "PASS" if observed == expected else "FAIL"
        method = SUBJECTS[scenario["method"]][0]
        result.setdefault(f"e0:l0:c0:m{method}", []).append(row)
    return result
