"""Decode and replay the fixture's original monitor regions and declarations."""

from hashlib import sha256
import json
from pathlib import Path

from dex_calls import method_code, method_ref
from dex_truth import Dex, U16, U32
from dex_witness import decode_instructions, exception_table, register_words


ROOT = Path(__file__).resolve().parent
DEX_SHA = "2f45fa2b44eb56719461c91fbf2fa8cc52bb1baf0dbc54334afd94dce9cd0055"
OWNER = "Lfixture/MonitorContract;"
SYNCHRONIZED, NATIVE, DECLARED = 0x20, 0x100, 0x20000
METHODS = {
    "metadataOnly": (1, "()Z", DECLARED, 0, 0),
    "nativeSync": (2, "()V", SYNCHRONIZED | NATIVE, 0, 0),
    "nested": (3, "(Z)Z", 0, 2, 4),
    "nestedNamed": (4, "(Ljava/lang/Object;Z)V", 0, 2, 4),
    "physical": (5, "(Z)Z", DECLARED, 1, 2),
}
SCENARIOS = {
    "metadata-only": "metadataOnly", "physical": "physical", "physical-fault": "physical",
    "nested": "nested", "nested-fault": "nested", "named": "nestedNamed", "named-fault": "nestedNamed",
}
HOLDS = ("Ljava/lang/Thread;", "holdsLock", "(Ljava/lang/Object;)Z")
HOOKS = {("Lfixture/MonitorHooks;", "probe", "(Ljava/lang/Object;Z)Z"),
         ("Lfixture/MonitorHooks;", "probeVoid", "(Ljava/lang/Object;Z)V")}


def class_methods(dex):
    offset = dex.classes.offset(0)
    owner = U32.unpack_from(dex.data, offset)[0]
    cursor = U32.unpack_from(dex.data, offset + 24)[0]
    if dex.descriptor(owner) != OWNER or cursor == 0:
        raise ValueError("the original class definition differs")
    counts = []
    for _ in range(4):
        count, cursor = dex.uleb(cursor)
        counts.append(count)
    if sum(counts[:2]) or sum(counts[2:]) != len(METHODS) + 1:
        raise ValueError("unexpected original monitor fixture member count")
    methods = {}
    for count in counts[2:]:
        index = 0
        for _ in range(count):
            delta, cursor = dex.uleb(cursor)
            access, cursor = dex.uleb(cursor)
            code, cursor = dex.uleb(cursor)
            index += delta
            if index in methods:
                raise ValueError("duplicate physical definition")
            methods[index] = (access, code)
    return methods


def bool_name(value):
    return str(value).lower()


def replay(dex, name, scenario, registers, inputs, instructions, protection):
    receiver = registers - inputs
    lock = receiver + int(name == "nestedNamed")
    slots = {registers - 1: scenario.endswith("-fault")} if inputs > 1 else {}
    fault = object()
    pending = None
    capture = None
    depth = 0
    events = ["before:false"]
    trace = []
    pc = 0
    for _ in range(32):
        insn = instructions[pc]
        op, register = insn.opcode, insn.word >> 8
        if op in (0x1d, 0x1e):
            if register != lock:
                raise ValueError("the original monitor changed operands")
            depth += 1 if op == 0x1d else -1
            if depth < 0:
                raise ValueError("the original monitor exits before an acquisition")
            trace.append({"pc": pc, "op": "enter" if op == 0x1d else "exit", "depth_after": depth})
        elif op == 0x71:
            target = method_ref(dex, insn.operands[0])
            args = register_words(insn)
            held = depth > 0
            trace.append({"pc": pc, "op": "invoke", "target": target, "args": args, "depth": depth})
            if target == HOLDS and args == (lock,):
                pending = held
            elif target in HOOKS and args == (lock, registers - 1):
                events.append(f"inside:{bool_name(held)}")
                if slots[args[1]]:
                    capture = fault
                    pc = next_catch(protection, pc)
                    if pc is None:
                        raise ValueError("modeled probe fault lacks its cleanup dispatch")
                    continue
                pending = held
            else:
                raise ValueError("unexpected original probe or argument")
        elif op == 0x0a:
            if type(pending) is not bool:
                raise ValueError("unexpected original move-result")
            slots[register], pending = pending, None
        elif op == 0x0d:
            if capture is None:
                raise ValueError("unexpected original move-exception")
            slots[register], capture = capture, None
        elif op == 0x27:
            capture = slots[register]
            trace.append({"pc": pc, "op": "throw", "exact": capture is fault, "depth": depth})
            destination = next_catch(protection, pc)
            if destination is not None:
                pc = destination
                continue
            if capture is not fault or depth:
                raise ValueError("original fault identity or monitor cleanup differs")
            events += ["throw:true", "after:false"]
            return events, trace
        elif op in (0x0e, 0x0f):
            if depth:
                raise ValueError("original method returns while retaining its monitor")
            result = "void" if op == 0x0e else bool_name(slots[register])
            trace.append({"pc": pc, "op": "return", "value": result})
            events += [f"return:{result}", "after:false"]
            return events, trace
        else:
            raise ValueError(f"unsupported original fixture opcode {op:#04x}")
        pc = insn.next
    raise ValueError("original fixture exceeded the instruction bound")


def next_catch(protection, pc):
    for item in protection:
        if item.start <= pc < item.end:
            if len(item.clauses) != 1 or item.clauses[0][0] is not None:
                raise ValueError("unexpected original fixture catch clause")
            return item.clauses[0][1]
    return None


def verify(artifact=ROOT / "classes.dex"):
    with Path(artifact).open("rb") as stream:
        data = stream.read(64 * 1024 + 1)
    if len(data) > 64 * 1024 or sha256(data).hexdigest() != DEX_SHA:
        raise ValueError("input differs from the independently authored monitor fixture")
    expected = json.loads((ROOT / "scenarios.json").read_text(encoding="utf-8"))
    if set(expected) != set(SCENARIOS):
        raise ValueError("authored monitor scenarios are incomplete")
    dex = Dex(data)
    declarations = class_methods(dex)
    results = {}
    for name, (index, descriptor, flags, enters, exits) in METHODS.items():
        access, code = declarations[index]
        member = f"{OWNER}->{name}{descriptor}"
        if (method_ref(dex, index) != (OWNER, name, descriptor)
                or access & (SYNCHRONIZED | NATIVE | DECLARED | 0x8) != flags):
            raise ValueError(f"original modifier or descriptor differs for {member}")
        result = {"at": f"e0:l0:c0:m{index}", "member": member, "access_flags": hex(access),
                  "declared_synchronized": bool(access & DECLARED), "checks": {},
                  "expected_classfile": {"acc_synchronized": bool(access & SYNCHRONIZED),
                                         "acc_native": bool(access & NATIVE),
                                         "monitorenter": enters, "monitorexit": exits}}
        results[name] = result
        if flags & NATIVE:
            if code or enters or exits:
                raise ValueError("native synchronization control unexpectedly has a body")
            continue
        if not code or method_code(dex, 0, index, member) != code:
            raise ValueError("missing original participant code")
        dex.data_range(code, 16, "original code header")
        registers, inputs, tries = (U16.unpack_from(dex.data, code + offset)[0] for offset in (0, 2, 6))
        units = U32.unpack_from(dex.data, code + 12)[0]
        num_inputs = {"()Z": 1, "(Z)Z": 2, "(Ljava/lang/Object;Z)V": 3}
        if not 0 < units <= 64 or not inputs == num_inputs[descriptor] <= registers <= 8:
            raise ValueError("unexpected original code or register extent")
        start = code + 16
        dex.data_range(start, 2 * units, "original code words")
        insns = decode_instructions(dex, start, units)
        protection = exception_table(dex, start + 2 * units, units, tries, insns)
        result["enter_pcs"] = [pc for pc, insn in insns.items() if insn.opcode == 0x1d]
        result["exit_pcs"] = [pc for pc, insn in insns.items() if insn.opcode == 0x1e]
        if (len(result["enter_pcs"]), len(result["exit_pcs"])) != (enters, exits):
            raise ValueError("original explicit monitor instruction count differs")
        for scenario, method in SCENARIOS.items():
            if method != name:
                continue
            observed, trace = replay(dex, name, scenario, registers, inputs, insns, protection)
            if observed != expected[scenario]:
                raise ValueError(f"raw scenario {scenario} disagrees with its authored trace")
            result["checks"][scenario] = {"expected": expected[scenario], "observed": observed, "trace": trace}
    return results


if __name__ == "__main__":
    report = verify()
    print(json.dumps(report, indent=2))
