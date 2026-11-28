"""Replay explicit DEX instance-field owners against independent object state."""

from argparse import ArgumentParser
from hashlib import sha256
import json
from pathlib import Path

from dex_calls import method_code, method_ref
from dex_truth import Dex, U16, U32
from dex_witness import decode_instructions, register_words, static_definition, static_field


ROOT = Path(__file__).resolve().parent
DEX_SHA = "a541c767b61d1400241b5fbf9c554c2a71c14808e04cb13e52ac4c1f5ae780f5"
FOREIGN = "Lfixture/ForeignFieldAccess;"
ANCESTOR = "Lfixture/RootFields;"
PARENT = "Lfixture/ParentFields;"
SHADOW = "Lfixture/ShadowFields;"
STORAGE = (ANCESTOR, PARENT, SHADOW)
CALLS = (FOREIGN, "calls", "I")
PICK = (FOREIGN, "pick", f"({SHADOW}){SHADOW}")
METHODS = {
    "readRootThroughShadow": (0, 1, FOREIGN, f"({SHADOW})I", 1, [("iget", ANCESTOR)]),
    "writeChosenRoot": (0, 2, FOREIGN, f"({SHADOW}I)V", 2,
                        [("invoke", FOREIGN), ("iput", ANCESTOR)]),
    "readOwn": (3, 6, SHADOW, "()I", 1, [("iget", SHADOW)]),
    "readParent": (3, 7, SHADOW, "()I", 1, [("iget", PARENT)]),
    "readRoot": (3, 8, SHADOW, "()I", 1, [("iget", ANCESTOR)]),
    "writeParent": (3, 9, SHADOW, "(I)V", 2, [("iput", PARENT)]),
    "writePeerRoot": (3, 10, SHADOW, f"({SHADOW}I)I", 3, [("iput", ANCESTOR), ("iget", ANCESTOR)]),
    "writeRoot": (3, 11, SHADOW, "(I)V", 2, [("iput", ANCESTOR)]),
}
SCENARIOS = {
    "own-read": ("readOwn", ("self",)),
    "parent-read": ("readParent", ("self",)),
    "root-read": ("readRoot", ("self",)),
    "parent-write": ("writeParent", ("self", 77)),
    "root-write": ("writeRoot", ("self", 77)),
    "peer-distinct": ("writePeerRoot", ("self", "peer", 77)),
    "peer-alias": ("writePeerRoot", ("self", "self", 77)),
    "peer-null": ("writePeerRoot", ("self", None, 77)),
    "external-read": ("readRootThroughShadow", ("self",)),
    "external-null": ("readRootThroughShadow", (None,)),
    "chosen-peer": ("writeChosenRoot", ("peer", 77)),
    "chosen-null": ("writeChosenRoot", (None, 77)),
}


def address(name):
    c, index, owner, signature, _, _ = METHODS[name]
    return f"e0:l0:c{c}:m{index}", f"{owner}->{name}{signature}"


def original_declarations(dex):
    result = []
    layout = [(FOREIGN, "Ljava/lang/Object;"), (ANCESTOR, "Ljava/lang/Object;"),
              (PARENT, ANCESTOR), (SHADOW, PARENT)]
    if dex.classes.count != len(layout):
        raise ValueError("unexpected original class count")
    for index, (owner, expected_super) in enumerate(layout):
        offset = dex.classes.offset(index)
        class_id, parent_id = (U32.unpack_from(dex.data, offset + field)[0] for field in (0, 8))
        if dex.descriptor(class_id) != owner or dex.descriptor(parent_id) != expected_super:
            raise ValueError("original subclass chain differs")
        start = U32.unpack_from(dex.data, offset + 24)[0]
        if not start:
            raise ValueError("original class data is missing")
        counts = []
        for _ in range(4):
            count, start = dex.uleb(start)
            counts.append(count)
        if counts[:2] != ([1, 0] if owner == FOREIGN else [0, 1]):
            raise ValueError("original field declaration count differs")
        field_id, start = dex.uleb(start)
        access, _ = dex.uleb(start)
        member = static_field(dex, field_id)
        expected_member, expected_access = (CALLS, 9) if owner == FOREIGN else ((owner, "value", "I"), 1)
        if access != expected_access or member != expected_member:
            raise ValueError("original field declaration identity differs")
        result.append({"class": owner, "super": expected_super, "fields": [member]})
    return result


def decode_body(dex, class_id, index, member, inputs):
    code = method_code(dex, class_id, index, member)
    if code % 4 or static_definition(dex, class_id, index) != (class_id == 0):
        raise ValueError(f"unexpected original method flags or alignment for {member}")
    dex.data_range(code, 16, "original selected method header")
    registers, encoded_inputs, tries = (U16.unpack_from(dex.data, code + offset)[0] for offset in (0, 2, 6))
    units = U32.unpack_from(dex.data, code + 12)[0]
    if tries or encoded_inputs != inputs or not inputs <= registers <= 6 or not 0 < units <= 16:
        raise ValueError(f"unsupported original code shape for {member}")
    start = code + 16
    dex.data_range(start, units * 2, "original selected code")
    return registers - inputs, decode_instructions(dex, start, units)


def receiver_helper(dex):
    owner, name, descriptor = PICK
    start, body = decode_body(dex, 0, 0, f"{owner}->{name}{descriptor}", 1)
    read, increment, write, returned = body.values()
    if ([insn.opcode for insn in body.values()] != [0x60, 0xd8, 0x67, 0x11]
            or read.word >> 8 or write.word >> 8 or increment.word >> 8
            or increment.operands != (0x100,) or returned.word >> 8 != start
            or any(static_field(dex, insn.operands[0]) != CALLS for insn in (read, write))):
        raise ValueError("receiver producer must increment calls once and return its argument")
    return {"at": "e0:l0:c0:m0", "member": f"{owner}->{name}{descriptor}",
            "read_pc": read.pc, "increment_pc": increment.pc, "write_pc": write.pc,
            "return_pc": returned.pc, "counter": CALLS}


def selected_body(dex, name):
    class_id, index, _, _, inputs, expected = METHODS[name]
    _, member = address(name)
    start, decoded = decode_body(dex, class_id, index, member, inputs)
    body = []
    sites = []
    for pc, insn in decoded.items():
        register = insn.word >> 8
        if insn.opcode in (0x52, 0x59):
            member = static_field(dex, insn.operands[0])
            if member[0] not in STORAGE or member[1:] != ("value", "I"):
                raise ValueError(f"unexpected original field reference {member}")
            operation = "iget" if insn.opcode == 0x52 else "iput"
            body.append({"pc": pc, "op": operation, "field": member,
                         "value": register & 0xf, "receiver": register >> 4})
            sites.append((operation, member[0]))
        elif insn.opcode == 0x71:
            target = method_ref(dex, insn.operands[0])
            if target != PICK or register_words(insn) != (start,):
                raise ValueError("unexpected original receiver producer")
            result = decoded[insn.next]
            if result.opcode != 0x0c:
                raise ValueError("receiver producer lacks its immediate result")
            body.append({"pc": pc, "op": "invoke", "target": target, "receiver": start})
            sites.append(("invoke", target[0]))
        elif insn.opcode == 0x0c:
            body.append({"pc": pc, "op": "move-result", "value": register})
        elif insn.opcode == 0x0f:
            body.append({"pc": pc, "op": "return", "value": register})
        elif insn.opcode == 0x0e:
            body.append({"pc": pc, "op": "return-void"})
        else:
            raise ValueError(f"unsupported original opcode {insn.opcode:#04x}")
    if sites != expected:
        raise ValueError(f"original field effect order differs for {name}")
    return start, body


def replay(start, body, args):
    state = {"self": dict(zip(STORAGE, (10, 20, 30))),
             "peer": dict(zip(STORAGE, (40, 50, 60)))}
    slots = {start + index: value for index, value in enumerate(args)}
    effects = []
    result = None
    pending = None
    calls = 0
    for action in body:
        op = action["op"]
        if op in ("iget", "iput"):
            receiver = slots[action["receiver"]]
            if receiver is None:
                effects.append({**action, "object": None, "fault": "NullPointerException"})
                result = "throw:NullPointerException"
                break
            cells = state[receiver]
            owner = action["field"][0]
            value = action["value"]
            if op == "iget":
                slots[value] = cells[owner]
                transferred = slots[value]
            else:
                transferred = slots[value]
                cells[owner] = transferred
            effects.append({**action, "object": receiver, "transferred": transferred})
        elif op == "invoke":
            calls += 1
            pending = slots[action["receiver"]]
            effects.append({**action, "object": pending, "calls": calls})
        elif op == "move-result":
            slots[action["value"]] = pending
        elif op == "return":
            result = f"return:{slots[action['value']]}"
            break
        elif op == "return-void":
            result = "return:void"
            break
    if result is None:
        raise ValueError("original selected path lacks a result")
    snapshots = [name + ":" + ":".join(str(state[name][owner]) for owner in STORAGE)
                 for name in ("self", "peer")]
    if any(action["op"] == "invoke" for action in body):
        snapshots.append(f"calls:{calls}")
    return [result, *snapshots], effects


def verify(artifact=ROOT / "classes.dex"):
    with Path(artifact).open("rb") as stream:
        data = stream.read(64 * 1024 + 1)
    if len(data) > 64 * 1024 or sha256(data).hexdigest() != DEX_SHA:
        raise ValueError("input differs from the authored ownership fixture")
    dex = Dex(data)
    declarations = original_declarations(dex)
    helper = receiver_helper(dex)
    methods = {name: selected_body(dex, name) for name in METHODS}
    expected = json.loads((ROOT / "scenarios.json").read_text(encoding="utf-8"))
    if set(expected) != set(SCENARIOS):
        raise ValueError("authored ownership scenario set differs")
    checks = []
    for scenario, wanted in expected.items():
        name, arguments = SCENARIOS[scenario]
        start, body = methods[name]
        observed, effects = replay(start, body, arguments)
        at, member = address(name)
        checks.append({"id": scenario, "at": at, "member": member, "expected": wanted,
                       "observed": observed, "status": "PASS" if observed == wanted else "SETUP_ERROR",
                       "trace": effects})
    return {"input_sha256": DEX_SHA, "declarations": declarations, "receiver_helper": helper, "checks": checks}


if __name__ == "__main__":
    parser = ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    report = verify()
    args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    for row in report["checks"]:
        print(row["at"], row["id"], row["status"], row["observed"])
    raise SystemExit(int(any(row["status"] != "PASS" for row in report["checks"])))
