"""Independently replay the authored DEX float/double comparisons and producers."""

import argparse
from dataclasses import dataclass
from hashlib import sha256
import json
from math import isnan
from pathlib import Path
from struct import unpack

from dex_calls import method_code, method_ref
from dex_truth import Dex, U16, U32
from dex_witness import decode_instructions, register_words, signed, static_definition, static_field


ROOT = Path(__file__).resolve().parent
DEX_SHA = "863f63955fe40eecc6709108530f0c9b23310f263b6263823d6a9b81d6668529"
OWNER = "Lfixture/FloatCompare;"
FAULT_TYPE = "Ljava/lang/RuntimeException;"
TRACE = (OWNER, "trace", "I")
METHODS = {
    "doubleFinite": (0, (), "Z"),
    "doubleHigh": (1, (FAULT_TYPE,), "I"),
    "doubleLeft": (2, ("Z",), "D"),
    "doubleLow": (3, (FAULT_TYPE,), "Z"),
    "doubleRightMaybe": (4, (FAULT_TYPE,), "D"),
    "left": (5, (), "F"),
    "leftNaN": (6, (), "F"),
    "lowFloatZero": (7, (FAULT_TYPE,), "Z"),
    "nanSkips": (8, (FAULT_TYPE,), "Z"),
    "one": (9, (), "Z"),
    "right": (10, (), "F"),
    "rightMaybe": (11, (FAULT_TYPE,), "F"),
    "value": (12, (), "I"),
    "zero": (13, (), "Z"),
}
SCENARIOS = {
    "finite-float-one": "one", "finite-float-zero": "zero", "finite-float-value": "value",
    "nan-float-high": "nanSkips", "nan-float-high-fault": "nanSkips",
    "nan-float-low": "lowFloatZero", "nan-float-low-fault": "lowFloatZero",
    "finite-double-one": "doubleFinite",
    "nan-double-high": "doubleHigh", "nan-double-high-fault": "doubleHigh",
    "nan-double-low": "doubleLow", "nan-double-low-fault": "doubleLow",
}
OPCODES = {
    0x2d: ("cmpl-float", "F", -1),
    0x2e: ("cmpg-float", "F", 1),
    0x2f: ("cmpl-double", "D", -1),
    0x30: ("cmpg-double", "D", 1),
}
CMP_LABELS = {entry[0] for entry in OPCODES.values()}
COMPARISONS = {
    "one": "cmpg-float", "zero": "cmpg-float", "value": "cmpg-float",
    "nanSkips": "cmpg-float", "lowFloatZero": "cmpl-float",
    "doubleFinite": "cmpg-double", "doubleHigh": "cmpg-double", "doubleLow": "cmpl-double",
}
WIDE_TAIL = object()


class Fault(Exception):
    """The exact caller-supplied exception in the authored model."""


@dataclass(frozen=True)
class Floating:
    width: str
    value: float


def address(method):
    index, args, returns = METHODS[method]
    return f"e0:l0:c0:m{index}", f"{OWNER}->{method}({''.join(args)}){returns}"


def instructions(dex, name):
    index, args, _ = METHODS[name]
    _, member = address(name)
    code = method_code(dex, 0, index, member)
    if code % 4 or not static_definition(dex, 0, index):
        raise ValueError(f"invalid original static code for {member}")
    dex.data_range(code, 16, "decoded DEX code header")
    registers, inputs, tries = (U16.unpack_from(dex.data, code + offset)[0] for offset in (0, 2, 6))
    count = U32.unpack_from(dex.data, code + 12)[0]
    if tries or not 0 < count <= 64 or inputs != len(args) or not inputs <= registers <= 10:
        raise ValueError(f"unexpected original code extents for {member}")
    start = code + 16
    dex.data_range(start, count * 2, "decoded DEX instructions")
    return registers, inputs, decode_instructions(dex, start, count)


def java_value(value, descriptor):
    if descriptor == "Z" and type(value) is int and value in (0, 1):
        return value
    if descriptor == "I" and type(value) is int:
        return value
    if descriptor in ("D", "F") and isinstance(value, Floating) and value.width == descriptor:
        return value
    if descriptor == FAULT_TYPE and (value is None or isinstance(value, Fault) or type(value) is int and value == 0):
        return None if value == 0 else value
    raise ValueError(f"original register value contradicts {descriptor}")


class Replay:
    def __init__(self, dex, methods):
        self.dex = dex
        self.methods = methods
        self.trace = 0
        self.events = []
        self.fault = Fault("authored exact right-operand exception")

    def call(self, name, args):
        registers, inputs, body = self.methods[name]
        _, types, returns = METHODS[name]
        if len(args) != inputs:
            raise ValueError("original invoke argument count differs")
        slots = {registers - inputs + index: java_value(value, descriptor)
                 for index, (value, descriptor) in enumerate(zip(args, types))}
        pc = 0
        pending = None
        for _ in range(32):
            insn = body[pc]
            op, register = insn.opcode, insn.word >> 8
            receipt = {"at": address(name)[0], "member": address(name)[1], "pc": pc}
            destination = insn.next
            if op == 0x71:
                owner, target, descriptor = method_ref(self.dex, insn.operands[0])
                if owner != OWNER or target not in self.methods or address(target)[1] != f"{owner}->{target}{descriptor}":
                    raise ValueError("unexpected original invoke target")
                if name not in COMPARISONS or target in COMPARISONS:
                    raise ValueError("unexpected nested producer call")
                self.events.append({**receipt, "op": "invoke", "target": target})
                pending = self.call(target, [slots[slot] for slot in register_words(insn)])
            elif op in (0x0a, 0x0b):
                width = "F" if op == 0x0a else "D"
                slots[register], pending = java_value(pending, width), None
                if op == 0x0b:
                    slots[register + 1] = WIDE_TAIL
            elif op in (0x60, 0x67):
                if static_field(self.dex, insn.operands[0]) != TRACE:
                    raise ValueError("original trace operand differs")
                if op == 0x60:
                    slots[register] = self.trace
                else:
                    self.trace = java_value(slots[register], "I")
                    self.events.append({**receipt, "op": "record", "trace": self.trace})
            elif op in (0xd8, 0xda):
                source = insn.operands[0] & 0xff
                immediate = signed(insn.operands[0] >> 8, 8)
                value = java_value(slots[source], "I")
                slots[register] = value + immediate if op == 0xd8 else value * immediate
            elif op == 0x12:
                slots[register & 0xf] = signed(register >> 4, 4)
            elif op in (0x15, 0x19):
                tail = bytes(2 if op == 0x15 else 6)
                payload = insn.operands[0].to_bytes(2, "big") + tail
                width = "F" if op == 0x15 else "D"
                slots[register] = Floating(width, unpack(">f" if width == "F" else ">d", payload)[0])
                if width == "D":
                    slots[register + 1] = WIDE_TAIL
            elif op in OPCODES:
                label, width, unordered = OPCODES[op]
                low, high = insn.operands[0] & 0xff, insn.operands[0] >> 8
                if width == "D" and (slots[low + 1] is not WIDE_TAIL or slots[high + 1] is not WIDE_TAIL):
                    raise ValueError("original double comparison lacks complete operand pairs")
                left, right = (java_value(slots[slot], width).value for slot in (low, high))
                result = unordered if isnan(left) or isnan(right) else (left > right) - (left < right)
                slots[register] = result
                self.events.append({**receipt, "op": label, "result": result, "unordered": isnan(left) or isnan(right)})
            elif op in (0x33, 0x38, 0x3b, 0x3d):
                if op == 0x33:
                    taken = slots[register & 0xf] != slots[register >> 4]
                elif op == 0x38:
                    taken = slots[register] is None or slots[register] == 0
                else:
                    value = java_value(slots[register], "I")
                    taken = value >= 0 if op == 0x3b else value <= 0
                destination = insn.target if taken else insn.next
                self.events.append({**receipt, "op": "branch", "taken": taken, "next": destination})
            elif op == 0x27:
                throwable = slots[register]
                if throwable is not self.fault:
                    raise ValueError("original throw did not use the supplied exception")
                self.events.append({**receipt, "op": "throw", "exact": True})
                raise throwable
            elif op in (0x0f, 0x10):
                if op == 0x10 and (returns != "D" or slots[register + 1] is not WIDE_TAIL):
                    raise ValueError("original wide return lacks a double pair")
                return java_value(slots[register], returns)
            else:
                raise ValueError(f"unsupported fixture opcode 0x{op:02x} at {name}:{pc}")
            pc = destination
        raise ValueError("original fixture exceeded the instruction bound")


def verify(artifact=ROOT / "classes.dex"):
    with Path(artifact).open("rb") as stream:
        data = stream.read(64 * 1024 + 1)
    if len(data) > 64 * 1024 or sha256(data).hexdigest() != DEX_SHA:
        raise ValueError("input differs from the independently authored float fixture")
    expected = json.loads((ROOT / "scenarios.json").read_text(encoding="utf-8"))
    if set(expected) != set(SCENARIOS):
        raise ValueError("authored floating-point scenarios are incomplete")
    dex = Dex(data)
    methods = {name: instructions(dex, name) for name in METHODS}
    for name, wanted in COMPARISONS.items():
        actual = [OPCODES[insn.opcode][0] for insn in methods[name][2].values() if insn.opcode in OPCODES]
        if actual != [wanted]:
            raise ValueError(f"unexpected original comparison in {name}")
    checks = []
    for scenario, wanted in expected.items():
        name = SCENARIOS[scenario]
        model = Replay(dex, methods)
        _, signature, output = METHODS[name]
        args = [model.fault if scenario.endswith("-fault") else None] if signature else []
        try:
            returned = model.call(name, args)
            value = str(bool(returned)).lower() if output == "Z" else str(returned)
            observed = f"return:{value}"
        except Fault as error:
            observed = f"throw:{str(error is model.fault).lower()}"
        observed = [observed, f"trace:{model.trace}"]
        statuses = [event["op"] for event in model.events if event["op"] in CMP_LABELS]
        correct_cmp = [] if scenario.endswith("-fault") else [COMPARISONS[name]]
        result = "PASS" if observed == wanted and statuses == correct_cmp else "SETUP_ERROR"
        at, member = address(name)
        checks.append({"id": scenario, "at": at, "member": member, "status": result,
                       "expected": wanted, "observed": observed, "trace": model.events})
    return {"input_sha256": DEX_SHA, "checks": checks}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    report = verify()
    args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    for row in report["checks"]:
        print(row["at"], row["id"], row["status"], row["observed"])
    raise SystemExit(int(any(row["status"] != "PASS" for row in report["checks"])))
