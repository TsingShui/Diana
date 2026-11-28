"""Replay authored DEX chain operations with text and reference identity."""

from dataclasses import replace
import hashlib
from pathlib import Path
from tempfile import TemporaryDirectory
from zipfile import ZIP_STORED, ZipFile, ZipInfo

from dex_truth import Dex, U32
from dex_witness import Execution, Instruction, Ref, Scenario, program, signed, static_field


OWNER = "Lfixture/BuilderIdentity;"
STRING = "Ljava/lang/String;"
FAMILIES = {"Ljava/lang/StringBuilder;", "Ljava/lang/StringBuffer;"}
SUFFIX = (OWNER, "SUFFIX", STRING)


class Recorder:
    def __init__(self, existing_string, suffix):
        self.events = []
        self.existing_string = existing_string
        self.suffix = suffix
        self.builders = {}
        self.allocations = []
        self.results = {}

    def allocate(self, owner):
        if owner not in FAMILIES:
            raise ValueError(f"unexpected allocation {owner}")
        value = Ref(f"builder-{len(self.allocations)}", owner)
        self.allocations.append(value)
        return value

    def static_field(self, target):
        if target == SUFFIX:
            return self.suffix
        raise ValueError(f"unexpected field {target}")

    def text(self, value):
        return self.results[value] if isinstance(value, Ref) else value

    def call(self, member, receiver, arguments):
        owner, name, descriptor = member
        if (owner, name, descriptor) == (OWNER, "observe", f"({STRING})V"):
            result, = arguments
            self.events.append(f"text:{self.text(result)}")
            self.events.append(f"identity:{'same' if result == self.existing_string else 'distinct'}")
            return None
        if owner not in FAMILIES or receiver not in self.allocations:
            raise ValueError(f"unexpected operation {member}")
        if name == "<init>" and descriptor in ("()V", f"({STRING})V"):
            self.builders[receiver] = self.text(arguments[0]) if arguments else ""
            return None
        if name == "toString" and descriptor == f"(){STRING}":
            value = Ref(f"string-{len(self.results)}", STRING)
            self.results[value] = self.builders[receiver]
            return value
        if name == "append" and descriptor in {
            f"({kind}){owner}" for kind in (STRING, "Ljava/lang/Object;", "I", "C", "Z")
        }:
            value, = arguments
            kind = descriptor.split(")", 1)[0][1:]
            if kind == "C":
                value = chr(value)
            elif kind == "Z":
                value = "true" if value else "false"
            elif kind == "I":
                value = str(value)
            self.builders[receiver] += self.text(value)
            return receiver
        raise ValueError(f"unsupported operation {member}")

    def finish(self, value, fault):
        if value is not None or fault is not None:
            raise ValueError("fixture did not complete normally")

def source_constant(dex):
    if dex.classes.count != 1 or dex.fields.count != 1 or static_field(dex, 0) != SUFFIX:
        raise ValueError("the original class does not establish the selected static string field")
    definition = dex.classes.offset(0)
    class_data, values = (U32.unpack_from(dex.data, definition + offset)[0] for offset in (24, 28))
    static_count, cursor = dex.uleb(class_data)
    instance_count, cursor = dex.uleb(cursor)
    _, cursor = dex.uleb(cursor)
    _, cursor = dex.uleb(cursor)
    field, cursor = dex.uleb(cursor)
    flags, _ = dex.uleb(cursor)
    if (static_count, instance_count, field) != (1, 0, 0) or flags & 0x18 != 0x18:
        raise ValueError("the original field is not the static final field in the value table")
    count, start = dex.uleb(values)
    dex.data_range(start, 1, "encoded static String")
    header = dex.data[start]
    width = (header >> 5) + 1
    if count != 1 or header & 0x1F != 0x17 or width > 4:
        raise ValueError("the original static field does not encode one String constant")
    dex.data_range(start + 1, width, "encoded static String index")
    return dex.string(int.from_bytes(dex.data[start + 1:start + 1 + width], "little"))


def prove_constant_prefix(selected):
    """Evaluate this fixture's effect-free const/add-int/lit8/int-to-byte prefix."""
    instructions = selected.instructions
    constant, addition, cast = (instructions[pc] for pc in (0, 2, 4))
    register = constant.word >> 8
    if (
        (constant.opcode, addition.opcode, cast.opcode) != (0x13, 0xD8, 0x8D)
        or (constant.next, addition.next, cast.next) != (2, 4, 5)
        or any(insn.target is not None and insn.target < 5 for insn in instructions.values())
        or selected.protection
        or addition.word >> 8 != register or addition.operands[0] & 0xFF != register
        or (cast.word >> 8 & 15, cast.word >> 12) != (register, register)
    ):
        raise ValueError("original arithmetic prefix differs from the bounded proof shape")
    initial = signed(constant.operands[0], 16)
    literal = signed(addition.operands[0] >> 8, 8)
    sum_word = signed((initial + literal) & 0xFFFFFFFF, 32)
    value = signed(sum_word & 0xFF, 8)
    replay = dict(instructions)
    replay[0] = Instruction(0, register << 8 | 0x13, (value & 0xFFFF,))
    for pc in (2, 3, 4):
        replay[pc] = Instruction(pc, 0x00, ())
    proof = [
        {"pc": constant.pc, "op": "const/16", "register": register, "value": initial},
        {"pc": addition.pc, "op": "add-int/lit8", "argument": literal, "value": sum_word},
        {"pc": cast.pc, "op": "int-to-byte", "value": value},
    ]
    return replace(selected, instructions=replay), proof


def verify(artifact, manifest, scenarios):
    with artifact.open("rb") as stream:
        data = stream.read(64 * 1024 + 1)
    if len(data) > 64 * 1024 or hashlib.sha256(data).hexdigest() != manifest["input_sha256"]:
        raise ValueError("artifact differs from the prepared identity fixture")
    suffix = source_constant(Dex(data))
    results = {}
    with TemporaryDirectory(prefix="decbench-builder-identity-") as scratch:
        archive = Path(scratch) / "original.zip"
        entry = ZipInfo("classes.dex", (1980, 1, 1, 0, 0, 0))
        entry.compress_type = ZIP_STORED
        with ZipFile(archive, "w") as wrapped:
            wrapped.writestr(entry, data)
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        for method in manifest["methods"]:
            at, member = method["at"], method["member"]
            name = member.split("->", 1)[1].split("(")[0]
            selected = program(archive, digest, at, member)
            prefix = []
            if name == "arithmeticBuffer":
                selected, prefix = prove_constant_prefix(selected)
            methods = []
            for row in scenarios[name]:
                model = lambda existing=row["arguments"][1]: Recorder(existing, suffix)
                args = tuple(map(int, row["arguments"][2:]))
                case = Scenario(row["id"], args, model, tuple(row["expected"]))
                execution = Execution(selected, case)
                trace = execution.run()
                execution.model.finish(*execution.outcome)
                measured = execution.model.events
                methods.append({
                    "id": row["id"], "status": "PASS" if measured == row["expected"] else "SETUP_ERROR",
                    "expected": row["expected"], "observed": measured, "trace": [*prefix, *trace],
                })
            results[at] = methods
    return results
