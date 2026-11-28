"""Compare exact rendered class behavior with raw DEX signed-zero traces."""

from hashlib import sha256
import json
from pathlib import Path
from subprocess import SubprocessError
from tempfile import TemporaryDirectory

from dex_calls import method_code, method_ref
from dex_truth import Dex, U16, U32
from dex_witness import decode_instructions, exception_table, register_words, signed, static_definition, static_field
from tool_capture import run


ROOT = Path(__file__).resolve().parent
HASH = "f25e5338bdcce0794546f501417b23ab629cc19108341f932856542001f86db8"
OWNER = "Lfixture/SignedSelections;"
ARITHMETIC = "Ljava/lang/ArithmeticException;"
METHODS = {
    "inspectFloat": (0, f"(F{ARITHMETIC})I", 2),
    "protectedUse": (1, f"(Z{ARITHMETIC})I", 2),
    "rawDouble": (2, "(Z)J", 1),
    "rawFloat": (3, "(Z)I", 1),
    "selectFloat": (4, "(Z)F", 1),
    "storeDouble": (5, "(Z)V", 1),
    "storeFloat": (6, "(Z)V", 1),
}
FIELDS = {"lastBits": "I", "caught": ARITHMETIC, "staticFloat": "F", "staticDouble": "D"}
JAVA = ["java", "-XX:+IgnoreUnrecognizedVMOptions", "-XX:-UsePerfData", "-Xmx128m"]
EXECUTION_ERRORS = (OSError, ValueError, KeyError, SubprocessError)


class FixtureFault(Exception):
    """The original catch observes the identity of this one modeled argument."""


def selected(dex, name):
    index, proto, args = METHODS[name]
    code = method_code(dex, 0, index, f"{OWNER}->{name}{proto}")
    if not static_definition(dex, 0, index):
        raise ValueError(f"{name} is not a static definition")
    dex.data_range(code, 16, "fixture code item header")
    registers, inputs, tries = (U16.unpack_from(dex.data, code + i)[0] for i in (0, 2, 6))
    units = U32.unpack_from(dex.data, code + 12)[0]
    if inputs != args or registers > 8 or inputs > registers or not 0 < units <= 64:
        raise ValueError(f"{name} exceeds the fixture register or code bound")
    start = code + 16
    dex.data_range(start, 2 * units, "fixture instructions")
    instructions = decode_instructions(dex, start, units)
    protection = exception_table(dex, start + 2 * units, units, tries, instructions)
    if name == "protectedUse":
        if len(protection) != 1 or len(protection[0].clauses) != 1:
            raise ValueError("expected one typed catch in protectedUse")
        region = protection[0]
        if (region.clauses[0][0] != ARITHMETIC
                or not region.start < region.end
                or instructions[region.start].opcode != 0x71):
            raise ValueError("protectedUse has a different protected call boundary")
    elif protection:
        raise ValueError(f"unexpected protected range in {name}")
    return registers - inputs, instructions, protection


def replay(dex, decoded, name, args, fields, trace):
    base, instructions, protection = decoded[name]
    slots = {base + index: value for index, value in enumerate(args)}
    pc, pending, caught = 0, None, None
    for _ in range(40):
        insn = instructions[pc]
        op, reg = insn.opcode, insn.word >> 8
        pc = insn.next
        if op == 0x12:
            slots[reg & 0x0f] = signed(reg >> 4, 4) & 0xffffffff
        elif op == 0x15:
            slots[reg] = insn.operands[0] << 16
        elif op in (0x16, 0x19):
            if op == 0x16:
                bits = signed(insn.operands[0], 16) & 0xffffffffffffffff
            else:
                bits = insn.operands[0] << 48
            slots[reg], slots[reg + 1] = bits & 0xffffffff, bits >> 32
        elif op == 0x38:
            pc = pc if slots[reg] else insn.target
            trace.append([name, insn.pc, "branch", pc])
        elif op == 0x28:
            pc = insn.target
        elif op == 0x71:
            binding = method_ref(dex, insn.operands[0])
            values = tuple(slots[r] for r in register_words(insn))
            trace.append([name, insn.pc, "call", list(binding), values[0]])
            if binding == ("Ljava/lang/Float;", "floatToRawIntBits", "(F)I") and len(values) == 1:
                pending = values[0]
            elif binding == ("Ljava/lang/Double;", "doubleToRawLongBits", "(D)J") and len(values) == 2:
                pending = values[0] | values[1] << 32
            elif name == "protectedUse" and binding == (OWNER, "inspectFloat", f"(F{ARITHMETIC})I"):
                try:
                    pending = replay(dex, decoded, "inspectFloat", values, fields, trace)
                except FixtureFault as fault:
                    handler = next((target for region in protection if region.start <= insn.pc < region.end
                                    for kind, target in region.clauses if kind == ARITHMETIC), None)
                    if handler is None:
                        raise ValueError("the helper fault escaped the original protected range") from fault
                    caught, pc = fault, handler
                    trace.append([name, insn.pc, "catch", handler])
            else:
                raise ValueError(f"unsupported original fixture call {binding}")
        elif op in (0x0a, 0x0b) and pending is not None:
            slots[reg] = pending & 0xffffffff
            if op == 0x0b:
                slots[reg + 1] = pending >> 32
            pending = None
        elif op == 0x0d and caught is not None:
            slots[reg], caught = caught, None
        elif op in (0x60, 0x67, 0x68, 0x69):
            owner, field, descriptor = static_field(dex, insn.operands[0])
            if owner != OWNER or FIELDS.get(field) != descriptor:
                raise ValueError("original field binding differs from the fixture")
            if op == 0x60 and field == "lastBits":
                slots[reg] = fields[field]
                trace.append([name, insn.pc, "read", field, slots[reg]])
            else:
                expected_op = {"I": 0x67, "F": 0x67, "D": 0x68, ARITHMETIC: 0x69}[descriptor]
                if op != expected_op:
                    raise ValueError("wrong width or operation for the original field")
                value = slots[reg] | slots[reg + 1] << 32 if op == 0x68 else slots[reg]
                fields[field] = value
                trace.append([name, insn.pc, "write", field, "fault" if isinstance(value, FixtureFault) else value])
        elif op == 0x27 and isinstance(slots[reg], FixtureFault):
            trace.append([name, insn.pc, "throw", ARITHMETIC])
            raise slots[reg]
        elif op == 0x0e:
            return None
        elif op == 0x0f:
            return slots[reg]
        elif op == 0x10:
            return slots[reg] | slots[reg + 1] << 32
        else:
            raise ValueError(f"unsupported original fixture opcode {op:#04x}")
    raise ValueError(f"{name} exceeded the instruction trace bound")


def original(artifact, scenarios):
    with artifact.open("rb") as stream:
        data = stream.read(16 * 1024 + 1)
    if len(data) > 16 * 1024 or sha256(data).hexdigest() != HASH:
        raise ValueError("original signed-zero DEX differs from the authored fixture")
    dex = Dex(data)
    decoded = {name: selected(dex, name) for name in METHODS}
    observations = {}
    for case, method, choice, has_fault, _ in scenarios:
        fault = FixtureFault() if has_fault else None
        args = (int(choice), fault) if method == "protectedUse" else (int(choice),)
        fields = {"lastBits": 7, "caught": None, "staticFloat": 0x3f800000,
                  "staticDouble": 0x3ff0000000000000}
        trace = []
        value = replay(dex, decoded, method, args, fields, trace)
        if method == "protectedUse":
            identity = "normal" if fields["caught"] is None else (
                "same-fault" if fields["caught"] is fault else "different-fault"
            )
            observed = f"{value:08x}:{fields['lastBits']:08x}:{identity}"
        elif method in ("storeFloat", "storeDouble"):
            field = "staticFloat" if method == "storeFloat" else "staticDouble"
            observed = f"{fields[field]:0{8 if method == 'storeFloat' else 16}x}"
        else:
            observed = f"{value:0{16 if method == 'rawDouble' else 8}x}"
        observations[case] = {"value": observed, "trace": trace}
    return observations


def call(argv, directory, timeout):
    result = run(argv, cwd=directory, timeout=timeout, output_limit=65536, diagnostic_limit=65536)
    if result.returncode:
        detail = (result.stderr or result.stdout).decode("utf-8", "replace")
        raise ValueError(detail.replace(str(directory), "<fixture>")[:1500]
                         or f"{Path(argv[0]).name} exited with status {result.returncode}")
    return result.stdout.decode("utf-8", "strict")


def execute(source, directory, scenarios, timeout):
    directory.mkdir()
    java = directory / "SignedSelections.java"
    java.write_text(source, encoding="utf-8")
    call([*JAVA, "-m", "jdk.compiler/com.sun.tools.javac.Main", "-proc:none", "-encoding",
          "UTF-8", "-d", directory, java, ROOT / "CheckSignedSelections.java"], directory, timeout)
    observed = {}
    for case, method, choice, fault, _ in scenarios:
        try:
            text = call([*JAVA, "-cp", directory, "fixture.CheckSignedSelections", method,
                         str(choice).lower(), str(fault).lower()], directory, timeout).splitlines()
            if len(text) != 1:
                raise ValueError("observer must publish exactly one line")
            observed[case] = (text[0], None)
        except EXECUTION_ERRORS as error:
            observed[case] = (None, str(error))
    return observed


def class_source(binary, artifact, scratch, timeout):
    response = call([binary, "get-class", artifact, OWNER, "--format", "json"], scratch, timeout)
    publication = json.loads(response)
    if (publication["schema_version"] != 5 or publication["command"] != "get-class"
            or publication["selection"]["class_at"] != f"e0:l0:c0::{OWNER}" or publication["issues"]):
        raise ValueError("complete SignedSelections class publication unavailable")
    text = publication["result"]["text"]
    if not isinstance(text, str) or not text.strip():
        raise ValueError("SignedSelections publication is empty")
    return text


def probe(binary, artifact, timeout):
    binary, artifact = Path(binary).resolve(), Path(artifact).resolve()
    scenarios = json.loads((ROOT / "scenarios.json").read_text(encoding="utf-8"))
    ids = {case for case, _, _, _, _ in scenarios}
    names = {method for _, method, _, _, _ in scenarios}
    if len(ids) != len(scenarios) or names != METHODS.keys() - {"inspectFloat"}:
        raise ValueError("scenario coverage differs from the authored fixture")
    raw = original(artifact, scenarios)
    recovered = {}
    candidate_error = None
    with TemporaryDirectory(prefix="signed-zero-dex-") as path:
        directory = Path(path)
        control = execute((ROOT / "SignedSelections.java").read_text(encoding="utf-8"),
                          directory / "authored", scenarios, timeout)
        try:
            text = class_source(binary, artifact, directory, timeout)
            recovered = execute(text, directory / "recovered", scenarios, timeout)
        except EXECUTION_ERRORS as error:
            candidate_error = str(error)
    results = {}
    for case, method, choice, fault, expected in scenarios:
        authored, control_error = control[case]
        observed, error = recovered.get(case, (None, candidate_error))
        row = {"id": case, "arguments": [choice, fault], "expected": expected, "observed": observed,
               "source_control": authored, "original_dex": raw[case]}
        if raw[case]["value"] != expected or authored != expected:
            row.update(status="SETUP_ERROR", detail=control_error or "original controls differ")
        elif observed is None:
            row.update(status="UNCHECKED", detail=error)
        else:
            row["status"] = "PASS" if observed == expected else "FAIL"
        results.setdefault(f"e0:l0:c0:m{METHODS[method][0]}", []).append(row)
    return results
