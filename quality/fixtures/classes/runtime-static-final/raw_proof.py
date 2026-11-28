"""Replay the retained field accesses and ordered class initializer with bounded hooks."""

import hashlib
import json
from pathlib import Path
import sys
from tempfile import TemporaryDirectory
from zipfile import ZIP_STORED, ZipFile, ZipInfo

from dex_calls import method_ref
from dex_truth import Dex, U32
from dex_witness import program, register_words, signed, static_field


ROOT = Path(__file__).resolve().parent
DEX_SHA = "c1da143c2c45f8e00b44ed1e381d03bbe25893695c79d33e05e75c3b27355101"
PROVIDER = "Lfixture/DexProvider;"
CONSUMER = "Lfixture/DexConsumer;"
HOOKS = "Lfixture/DexInitEvents;"
STRING = "Ljava/lang/String;"
MEMBERS = {
    "compileNumber": (0, 0, f"{CONSUMER}->compileNumber()I"),
    "compileText": (0, 1, f"{CONSUMER}->compileText(){STRING}"),
    "runtimeNumber": (0, 2, f"{CONSUMER}->runtimeNumber()I"),
    "runtimeText": (0, 3, f"{CONSUMER}->runtimeText(){STRING}"),
    "<clinit>": (2, 6, f"{PROVIDER}-><clinit>()V"),
}


def original_static_fields(dex):
    base = dex.classes.offset(2)
    owner = dex.descriptor(U32.unpack_from(dex.data, base)[0])
    start, values = (U32.unpack_from(dex.data, base + offset)[0] for offset in (24, 28))
    if owner != PROVIDER or not start or not values:
        raise ValueError("unable to locate the original provider field and value tables")
    counts = []
    for _ in range(4):
        count, start = dex.uleb(start)
        counts.append(count)
    if counts[:2] != [4, 0]:
        raise ValueError("unexpected provider field count")
    ordered = []
    index = 0
    for _ in range(counts[0]):
        delta, start = dex.uleb(start)
        flags, start = dex.uleb(start)
        index += delta
        member = static_field(dex, index)
        if member[0] != PROVIDER or flags & 0x18 != 0x18:
            raise ValueError("selected field is not the original static final member")
        ordered.append(member)
    count, cursor = dex.uleb(values)
    if count != 2 or len(set(ordered)) != 4:
        raise ValueError("expected two encoded initial values and two blank finals")
    fields = {}
    for index, member in enumerate(ordered):
        encoded = index < count
        if encoded:
            dex.data_range(cursor, 1, "encoded provider field value")
            marker = dex.data[cursor]
            width = (marker >> 5) + 1
            kind = marker & 0x1f
            if width > 4 or (kind, member[2]) not in ((0x17, STRING), (0x04, "I")):
                raise ValueError("unexpected provider field literal type or width")
            dex.data_range(cursor + 1, width, "encoded provider field value payload")
            data = dex.data[cursor + 1:cursor + 1 + width]
            value = sys.intern(dex.string(int.from_bytes(data, "little"))) if kind == 0x17 else (
                int.from_bytes(data, "little", signed=True)
            )
            cursor += 1 + width
        else:
            value = None if member[2] == STRING else 0
        name = member[1]
        if encoded != name.startswith("COMPILE_"):
            raise ValueError("original encoded-value ordering does not match the authored controls")
        fields[member] = {"encoded": encoded, "value": value}
    return fields


class InitFailure(Exception):
    def __init__(self, cause):
        super().__init__("modeled initialization failure")
        self.cause = cause


class PriorInitFailure(Exception):
    pass


class RawState:
    def __init__(self, selected, fields, fail):
        self.selected = selected
        self.fields = {member: info["value"] for member, info in fields.items()}
        self.metadata = fields
        self.fail = fail
        self.events = []
        self.trace = []
        self.state = "cold"
        self.fault = RuntimeError("modeled fixture hook failure")

    def read_static(self, field):
        if field not in self.fields:
            raise ValueError(f"unexpected static operand {field}")
        if self.state == "erroneous":
            raise PriorInitFailure()
        if self.state == "cold":
            self.state = "initializing"
            try:
                self.execute("<clinit>")
            except RuntimeError as cause:
                if cause is not self.fault:
                    raise
                self.state = "erroneous"
                raise InitFailure(cause) from cause
            self.state = "initialized"
        return self.fields[field]

    def execute(self, name):
        selected = self.selected[name]
        slots = {}
        pc = 0
        for _ in range(40):
            insn = selected.instructions[pc]
            opcode, word = insn.opcode, insn.word
            register = word >> 8
            receipt = {"method": name, "pc": pc}
            if opcode == 0x13:
                slots[register] = signed(insn.operands[0], 16)
                self.trace.append({**receipt, "op": "const", "value": slots[register]})
            elif opcode == 0x1a:
                slots[register] = sys.intern(selected.dex.string(insn.operands[0]))
                self.trace.append({**receipt, "op": "const-string", "value": slots[register]})
            elif opcode in (0x60, 0x62, 0x67, 0x69):
                field = static_field(selected.dex, insn.operands[0])
                if field[2] != ("I" if opcode in (0x60, 0x67) else STRING):
                    raise ValueError("selected static opcode disagrees with the field descriptor")
                if opcode in (0x60, 0x62):
                    self.trace.append({**receipt, "op": "sget", "field": field})
                    slots[register] = self.read_static(field)
                else:
                    if self.state != "initializing" or name != "<clinit>" or self.metadata[field]["encoded"]:
                        raise ValueError("unexpected write to original provider field")
                    self.fields[field] = slots[register]
                    self.trace.append({**receipt, "op": "sput", "field": field, "value": slots[register]})
            elif opcode == 0x71:
                member = method_ref(selected.dex, insn.operands[0])
                args = [slots[number] for number in register_words(insn)]
                self.trace.append({**receipt, "op": "invoke", "member": member, "args": args})
                if member == (HOOKS, "begin", f"({STRING}I)V"):
                    text, number = args
                    self.events.append(f"begin:{text}:{number}")
                elif member == (HOOKS, "complete", "()V") and not args:
                    if self.fail:
                        self.events.append("initializer-fault")
                        raise self.fault
                    self.events.append("complete")
                else:
                    raise ValueError(f"unexpected initializer hook {member}")
            elif opcode == 0x0e:
                return None
            elif opcode in (0x0f, 0x11):
                return slots[register]
            else:
                raise ValueError(f"unsupported original fixture opcode 0x{opcode:02x}")
            pc = insn.next
        raise ValueError("raw fixture path exceeds its instruction bound")

    def compile_controls(self):
        text = self.execute("compileText")
        number = self.execute("compileNumber")
        true_text = text is self.metadata[(PROVIDER, "COMPILE_TEXT", STRING)]["value"]
        if self.metadata[(PROVIDER, "COMPILE_NUMBER", "I")]["value"] != number:
            raise ValueError("literal number read disagrees with the original encoded field")
        self.events.append(f"compile:{str(true_text).lower()}:{number}")

    def read(self, kind, order):
        self.events.append(f"{order}:{kind}")
        try:
            result = self.execute("runtimeText" if kind == "text" else "runtimeNumber")
            if kind == "text":
                identity = result is sys.intern("runtime")
                self.events.append(f"value:text:{result}:{str(identity).lower()}")
            else:
                self.events.append(f"value:number:{result}")
        except InitFailure as error:
            self.events.append(f"error:initializer:{str(error.cause is self.fault).lower()}")
        except PriorInitFailure:
            self.events.append("error:prior-initializer")


def verify(artifact=ROOT / "classes.dex"):
    with Path(artifact).open("rb") as stream:
        data = stream.read(64 * 1024 + 1)
    if len(data) > 64 * 1024 or hashlib.sha256(data).hexdigest() != DEX_SHA:
        raise ValueError("original fixture input differs from the prepared DEX")
    fields = original_static_fields(Dex(data))
    rows = []
    with TemporaryDirectory(prefix="blank-final-dex-raw-") as temp:
        archive = Path(temp) / "original.zip"
        entry = ZipInfo("classes.dex", (1980, 1, 1, 0, 0, 0))
        entry.compress_type = ZIP_STORED
        with ZipFile(archive, "w") as wrapped:
            wrapped.writestr(entry, data)
        sha256 = hashlib.sha256(archive.read_bytes()).hexdigest()
        selected = {
            name: program(archive, sha256, f"e0:l0:c{owner}:m{method}", member)
            for name, (owner, method, member) in MEMBERS.items()
        }
        expected = json.loads((ROOT / "scenarios.json").read_text())
        for scenario, wanted in expected.items():
            raw = RawState(selected, fields, scenario.startswith("fail-"))
            raw.compile_controls()
            if scenario != "compile-only":
                first, second = ("text", "number") if scenario.endswith("-text") else ("number", "text")
                raw.read(first, "first")
                raw.read(second, "second")
                raw.compile_controls()
            rows.append({
                "id": scenario, "status": "PASS" if wanted == raw.events else "SETUP_ERROR",
                "expected": wanted, "observed": raw.events, "trace": raw.trace,
            })
    return {"input_sha256": DEX_SHA, "fields": [
        {"member": member, **details} for member, details in fields.items()
    ], "checks": rows}


if __name__ == "__main__":
    report = verify()
    print(json.dumps(report, indent=2))
    assert all(row["status"] == "PASS" for row in report["checks"])
