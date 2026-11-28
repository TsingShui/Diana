"""Replay exact DEX interface initialization and modeled same-thread reentry."""

from hashlib import sha256
import json
from pathlib import Path
from tempfile import TemporaryDirectory
from zipfile import ZipFile, ZipInfo

from dex_calls import method_ref
from dex_truth import Dex, U32
from dex_witness import program, register_words, static_field


ROOT = Path(__file__).resolve().parent
DEX_SHA = "73764c40ac9c7726a5651101264c580d34c7c9cafe110dc218500c09eb225c08"
FACE = "Lfixture/RuntimeFace;"
CONSUMER = "Lfixture/FaceConsumer;"
STRING = "Ljava/lang/String;"
HOOK = ("Lfixture/InterfaceEffects;", "second", f"(){STRING}")
FIRST, SECOND = (FACE, "FIRST", STRING), (FACE, "SECOND", STRING)
MEMBERS = {
    "first": ("e0:l0:c0:m0", f"{CONSUMER}->first{HOOK[2]}"),
    "second": ("e0:l0:c0:m1", f"{CONSUMER}->second{HOOK[2]}"),
    "<clinit>": ("e0:l0:c2:m3", f"{FACE}-><clinit>()V"),
    "readFirst": ("e0:l0:c2:m4", f"{FACE}->readFirst{HOOK[2]}"),
    "readSecond": ("e0:l0:c2:m5", f"{FACE}->readSecond{HOOK[2]}"),
}


def field_metadata(dex):
    base = dex.classes.offset(2)
    owner, flags = (U32.unpack_from(dex.data, base + offset)[0] for offset in (0, 4))
    start, encoded = (U32.unpack_from(dex.data, base + offset)[0] for offset in (24, 28))
    if dex.descriptor(owner) != FACE or flags & 0x200 == 0 or not start or encoded:
        raise ValueError("expected an interface with no encoded field initial values")
    static_count, start = dex.uleb(start)
    instance_count, start = dex.uleb(start)
    if (static_count, instance_count) != (2, 0):
        raise ValueError("unexpected original field count")
    for _ in range(2):
        _, start = dex.uleb(start)
    members = {}
    field_index = 0
    for _ in range(static_count):
        delta, start = dex.uleb(start)
        flags, start = dex.uleb(start)
        field_index += delta
        if flags & 0x19 != 0x19:
            raise ValueError("expected a public static final interface field")
        members[static_field(dex, field_index)] = flags
    if set(members) != {FIRST, SECOND}:
        raise ValueError("original interface field descriptors differ")
    return [{"member": member, "access_flags": flags, "encoded": False}
            for member, flags in sorted(members.items())]


class Replay:
    def __init__(self, methods):
        self.methods = methods
        self.fields = {FIRST: None, SECOND: None}
        self.initialization = "cold"
        self.events = []
        self.trace = []

    def run(self, name):
        method = self.methods[name]
        registers = {}
        pending = None
        pc = 0
        for _ in range(12):
            insn = method.instructions[pc]
            op = insn.opcode
            register = insn.word >> 8
            receipt = {"at": MEMBERS[name][0], "member": MEMBERS[name][1], "pc": pc}
            if op == 0x1a:
                registers[register] = method.dex.string(insn.operands[0])
                self.trace.append({**receipt, "op": "const-string", "value": registers[register]})
            elif op in (0x62, 0x69):
                field = static_field(method.dex, insn.operands[0])
                if field not in self.fields:
                    raise ValueError("unexpected original field reference")
                if op == 0x62:
                    self.trace.append({**receipt, "op": "sget-object", "field": field})
                    if self.initialization == "cold":
                        self.initialization = "active"
                        self.run("<clinit>")
                        self.initialization = "complete"
                    registers[register] = self.fields[field]
                else:
                    if name != "<clinit>" or self.initialization != "active":
                        raise ValueError("unexpected write outside interface initialization")
                    self.fields[field] = registers[register]
                    self.trace.append({**receipt, "op": "sput-object", "field": field,
                                       "value": registers[register]})
            elif op == 0x71:
                target = method_ref(method.dex, insn.operands[0])
                if target != HOOK or register_words(insn) or self.initialization != "active":
                    raise ValueError("unexpected original foreign callback")
                self.trace.append({**receipt, "op": "invoke-static", "target": target})
                first, second = (self.run(getter) for getter in ("readFirst", "readSecond"))
                self.events.append(f"during:{display(first)}:{display(second)}")
                pending = "second"
            elif op == 0x0c:
                if pending is None:
                    raise ValueError("original move-result-object has no modeled callback value")
                registers[register], pending = pending, None
            elif op == 0x11:
                self.trace.append({**receipt, "op": "return-object", "value": registers[register]})
                return registers[register]
            elif op == 0x0e and name == "<clinit>":
                self.trace.append({**receipt, "op": "return-void"})
                return None
            else:
                raise ValueError(f"unsupported original fixture opcode 0x{op:02x}")
            pc = insn.next
        raise ValueError("original method exceeded the fixture instruction bound")


def display(value):
    return "null" if value is None else value


def verify(artifact=ROOT / "classes.dex"):
    with Path(artifact).open("rb") as original:
        data = original.read(64 * 1024 + 1)
    if len(data) > 64 * 1024 or sha256(data).hexdigest() != DEX_SHA:
        raise ValueError("input differs from the independently authored interface fixture")
    expected = json.loads((ROOT / "scenarios.json").read_text(encoding="utf-8"))
    fields = field_metadata(Dex(data))
    checks = []
    with TemporaryDirectory(prefix="interface-dex-raw-") as temporary:
        archive = Path(temporary) / "original.zip"
        with ZipFile(archive, "w") as equivalent:
            equivalent.writestr(ZipInfo("classes.dex", (1980, 1, 1, 0, 0, 0)), data)
        checksum = sha256(archive.read_bytes()).hexdigest()
        methods = {name: program(archive, checksum, at, member)
                   for name, (at, member) in MEMBERS.items()}
        if any(method.protection or method.inputs for method in methods.values()):
            raise ValueError("unexpected original protection or inputs")
        for scenario, wanted in expected.items():
            state = Replay(methods)
            state.events.append("before")
            selected = state.run(scenario)
            state.events.append(f"selected:{display(selected)}")
            state.events.append(f"first:{display(state.run('readFirst'))}")
            state.events.append(f"second:{display(state.run('readSecond'))}")
            checks.append({"id": scenario, "status": "PASS" if state.events == wanted else "SETUP_ERROR",
                           "expected": wanted, "observed": state.events, "trace": state.trace})
    return {"input_sha256": DEX_SHA, "fields": fields, "checks": checks}


if __name__ == "__main__":
    report = verify()
    print(json.dumps(report, indent=2))
    raise SystemExit(int(any(row["status"] != "PASS" for row in report["checks"])))
