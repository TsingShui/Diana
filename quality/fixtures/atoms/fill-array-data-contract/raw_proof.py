"""Decode the exact fill-array-data payloads and replay the authored DEX paths."""

from argparse import ArgumentParser
from hashlib import sha256
import json
from pathlib import Path

from dex_calls import method_code, method_ref
from dex_truth import Dex, U16, U32
from dex_witness import Instruction, exception_table, register_words, signed, static_definition, static_field


ROOT = Path(__file__).resolve().parent
DEX_SHA = "7085797575f054fe80f32a9f4b00f9ae63bddb58fb1e64e91c29f07936253510"
OWNER = "Lfixture/FillData;"
ARITHMETIC = "Ljava/lang/ArithmeticException;"
BOUNDS = "Ljava/lang/ArrayIndexOutOfBoundsException;"
NULL = "Ljava/lang/NullPointerException;"
FIELDS = {("trace", "I"), ("shared", "[I"), ("caught", ARITHMETIC)}
METHODS = {
    "fillEmpty": (0, "([I)V", 1),
    "fillFloat": (1, "([F)V", 1),
    "fillFloatBits": (2, "([F)V", 1),
    "fillFromReceiver": (3, "()V", 0),
    "fillTwo": (4, "([I)V", 1),
    "protectedFill": (5, f"([I{ARITHMETIC})I", 2),
    "receiver": (6, "()[I", 0),
    "receiverMaybe": (7, f"([I{ARITHMETIC})[I", 2),
}
SCENARIOS = {
    "short": "fillTwo", "null": "fillTwo", "int-prefix": "fillTwo",
    "empty": "fillEmpty", "empty-array": "fillEmpty", "empty-with-tail": "fillEmpty",
    "receiver": "fillFromReceiver", "receiver-null": "fillFromReceiver",
    "floats": "fillFloat", "float-bits": "fillFloatBits",
    "protected-normal": "protectedFill", "protected-short": "protectedFill",
    "protected-null": "protectedFill", "protected-fault": "protectedFill",
}
PAYLOADS = {
    "fillEmpty": [], "fillFloat": [0x3fc00000, 0x80000000],
    "fillFloatBits": [0, 0x80000000, 0x7fc12345],
    "fillFromReceiver": [11, 22], "fillTwo": [11, 22], "protectedFill": [11, 22],
}
WIDTHS = {
    0x00: 1, 0x0c: 1, 0x0d: 1, 0x0e: 1, 0x0f: 1, 0x11: 1, 0x12: 1,
    0x26: 3, 0x27: 1, 0x38: 2, 0x44: 2, 0x60: 2, 0x62: 2, 0x67: 2,
    0x69: 2, 0x71: 3, 0xd8: 2,
}


class Fault(Exception):
    def __init__(self, descriptor):
        self.descriptor = descriptor


def address(name):
    index, descriptor, _ = METHODS[name]
    return f"e0:l0:c0:m{index}", f"{OWNER}->{name}{descriptor}"


def decode(dex, name):
    index, _, inputs = METHODS[name]
    _, member = address(name)
    code = method_code(dex, 0, index, member)
    if code % 4 or not static_definition(dex, 0, index):
        raise ValueError("unexpected original code alignment or static flags")
    dex.data_range(code, 16, "original selected code header")
    registers, original_inputs, tries = (U16.unpack_from(dex.data, code + offset)[0] for offset in (0, 2, 6))
    units = U32.unpack_from(dex.data, code + 12)[0]
    if inputs != original_inputs or not inputs <= registers <= 8 or not 0 < units <= 64:
        raise ValueError(f"original fixture code extent differs at {member}")
    start = code + 16
    end = start + units * 2
    dex.data_range(start, units * 2, "original code words")
    words = [U16.unpack_from(dex.data, start + pc * 2)[0] for pc in range(units)]
    instructions = {}
    data = {}
    pc = 0
    while pc < units:
        word = words[pc]
        if word == 0x0300:
            if pc % 2 or pc + 4 > units:
                raise ValueError("unaligned or truncated original fill-array-data header")
            width, low, high = words[pc + 1:pc + 4]
            count = low | high << 16
            if width not in (1, 2, 4, 8) or count > 16:
                raise ValueError("original payload exceeds its element bounds")
            size = width * count
            payload_end = pc + 4 + (size + 1) // 2
            if payload_end > units:
                raise ValueError("original array payload exceeds its method")
            first = start + (pc + 4) * 2
            elements = [int.from_bytes(dex.data[first + i:first + i + width], "little")
                        for i in range(0, size, width)]
            data[pc] = {"width": width, "count": count, "elements": elements}
            pc = payload_end
            continue
        op = word & 0xff
        width = WIDTHS.get(op)
        if width is None or pc + width > units or op == 0 and word:
            raise ValueError(f"invalid fixture opcode {op:#04x} at {member}:{pc}")
        operands = tuple(words[pc + 1:pc + width])
        target = None
        if op == 0x26:
            target = pc + signed(operands[0] | operands[1] << 16, 32)
        elif op == 0x38:
            target = pc + signed(operands[0], 16)
        instructions[pc] = Instruction(pc, word, operands, target)
        pc += width
    for insn in instructions.values():
        destinations = data if insn.opcode == 0x26 else instructions
        if insn.target is not None and insn.target not in destinations:
            raise ValueError("original branch or payload target is absent")
    fills = [insn for insn in instructions.values() if insn.opcode == 0x26]
    if name in PAYLOADS:
        if len(fills) != 1 or len(data) != 1:
            raise ValueError(f"expected one original fill and one payload at {member}")
        payload = data[fills[0].target]
        if payload["width"] != 4 or payload["elements"] != PAYLOADS[name]:
            raise ValueError(f"original payload data differs at {member}")
    elif fills or data:
        raise ValueError("unexpected original payload in receiver method")
    protection = exception_table(dex, end, units, tries, instructions)
    if name == "protectedFill":
        if len(protection) != 1 or [kind for kind, _ in protection[0].clauses] != [BOUNDS, ARITHMETIC]:
            raise ValueError("original selected catch clauses differ")
    elif protection:
        raise ValueError("unexpected original protection in simple method")
    return registers - inputs, instructions, data, protection


class Replay:
    def __init__(self, dex, methods, shared):
        self.dex = dex
        self.methods = methods
        self.fields = {"trace": 0, "shared": shared, "caught": None}
        self.fault = Fault(ARITHMETIC)
        self.events = []

    def run(self, name, args):
        start, code, payloads, protection = self.methods[name]
        if len(args) != METHODS[name][2]:
            raise ValueError("unexpected original argument count")
        slots = {start + index: value for index, value in enumerate(args)}
        pc = 0
        pending = None
        caught = None
        for _ in range(32):
            insn = code[pc]
            position, member = address(name)
            at = {"at": position, "member": member, "pc": pc}
            op, register = insn.opcode, insn.word >> 8
            try:
                if op == 0x71:
                    owner, target, proto = method_ref(self.dex, insn.operands[0])
                    if (target not in ("receiver", "receiverMaybe") or name not in PAYLOADS
                            or address(target)[1] != f"{owner}->{target}{proto}"):
                        raise ValueError("unexpected original invocation")
                    self.events.append({**at, "op": "invoke-static", "target": target})
                    pending = self.run(target, [slots[slot] for slot in register_words(insn)])
                elif op == 0x0c:
                    slots[register], pending = pending, None
                elif op == 0x0d:
                    if caught is None:
                        raise ValueError("original catch has no exception")
                    slots[register], caught = caught, None
                elif op in (0x60, 0x62, 0x67, 0x69):
                    owner, field, descriptor = static_field(self.dex, insn.operands[0])
                    if owner != OWNER or (field, descriptor) not in FIELDS:
                        raise ValueError("unexpected fixture field operand")
                    if op in (0x60, 0x62):
                        slots[register] = self.fields[field]
                    else:
                        self.fields[field] = slots[register]
                        effect = {"value": slots[register]} if field == "trace" else {"exact": slots[register] is self.fault}
                        self.events.append({**at, "op": "sput", "field": field, **effect})
                elif op == 0xd8:
                    source, literal = insn.operands[0] & 0xff, signed(insn.operands[0] >> 8, 8)
                    slots[register] = slots[source] + literal
                elif op == 0x12:
                    slots[register & 0xf] = signed(register >> 4, 4)
                elif op == 0x38:
                    if slots[register] is None:
                        pc = insn.target
                        continue
                elif op == 0x27:
                    if slots[register] is not self.fault:
                        raise ValueError("original receiver throws a different exception")
                    raise self.fault
                elif op == 0x26:
                    target = slots[register]
                    payload = payloads[insn.target]
                    self.events.append({**at, "op": "fill-array-data", "payload_pc": insn.target,
                                        "length": len(target) if target is not None else None, **payload})
                    if target is None:
                        raise Fault(NULL)
                    if payload["count"] > len(target):
                        raise Fault(BOUNDS)
                    target[:payload["count"]] = payload["elements"]
                elif op == 0x44:
                    array, index = insn.operands[0] & 0xff, insn.operands[0] >> 8
                    target, offset = slots[array], slots[index]
                    if target is None:
                        raise Fault(NULL)
                    if not 0 <= offset < len(target):
                        raise Fault(BOUNDS)
                    slots[register] = target[offset]
                    self.events.append({**at, "op": "aget", "value": slots[register]})
                elif op in (0x0f, 0x11):
                    return slots[register]
                elif op == 0x0e:
                    return None
                elif op != 0x00:
                    raise ValueError(f"unsupported original opcode {op:#04x}")
            except Fault as error:
                self.events.append({**at, "op": "fault", "type": error.descriptor, "exact": error is self.fault})
                destination = next((handler for entry in protection if entry.start <= pc < entry.end
                                    for tag, handler in entry.clauses if tag == error.descriptor), None)
                if destination is None:
                    raise
                caught, pc = error, destination
                continue
            pc = insn.next
        raise ValueError("original fixture path exceeded the instruction bound")


def replay_scenario(dex, methods, scenario):
    if scenario in ("short", "protected-short"):
        ints = [7]
    elif scenario in ("empty", "null", "protected-null", "receiver-null", "floats", "float-bits"):
        ints = None
    elif scenario == "empty-array":
        ints = []
    else:
        ints = [7, 8, 9, 10]
    floats = {"floats": [0x40e00000, 0x41000000, 0x41100000],
              "float-bits": [0x40e00000, 0x41000000, 0x41100000, 0x41200000]}.get(scenario)
    model = Replay(dex, methods, ints)
    name = SCENARIOS[scenario]
    args = []
    if name == "protectedFill":
        args = [ints, model.fault if scenario == "protected-fault" else None]
    elif METHODS[name][2]:
        args = [floats if floats is not None else ints]
    try:
        result = model.run(name, args)
        outcome = f"return:{'void' if result is None else result}"
    except Fault as error:
        outcome = "throw:" + error.descriptor.rsplit("/", 1)[-1].removesuffix(";")
    if floats is not None:
        contents = "/".join(f"{bits:08x}" for bits in floats)
    else:
        contents = "null" if ints is None else "/".join(map(str, ints))
    caught = model.fields["caught"]
    observed = [outcome, f"contents:{contents}", f"trace:{model.fields['trace']}",
                "caught:" + ("none" if caught is None else "exact" if caught is model.fault else "other"),
                f"alias:{str(model.fields['shared'] is ints).lower()}"]
    return observed, model.events


def verify(artifact=ROOT / "classes.dex"):
    with Path(artifact).open("rb") as stream:
        data = stream.read(64 * 1024 + 1)
    if len(data) > 64 * 1024 or sha256(data).hexdigest() != DEX_SHA:
        raise ValueError("input differs from the prepared array data fixture")
    expected = json.loads((ROOT / "scenarios.json").read_text(encoding="utf-8"))
    if set(expected) != set(SCENARIOS):
        raise ValueError("authored array data scenario contract differs")
    dex = Dex(data)
    methods = {name: decode(dex, name) for name in METHODS}
    rows = []
    for scenario, wanted in expected.items():
        observed, trace = replay_scenario(dex, methods, scenario)
        at, member = address(SCENARIOS[scenario])
        rows.append({"id": scenario, "at": at, "member": member, "expected": wanted,
                     "observed": observed, "trace": trace,
                     "status": "PASS" if observed == wanted else "SETUP_ERROR"})
    return {"input_sha256": DEX_SHA, "checks": rows}


if __name__ == "__main__":
    parser = ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    proof = verify()
    args.output.write_text(json.dumps(proof, indent=2) + "\n", encoding="utf-8")
    for row in proof["checks"]:
        print(row["at"], row["id"], row["status"], row["observed"])
    raise SystemExit(int(any(row["status"] != "PASS" for row in proof["checks"])))
