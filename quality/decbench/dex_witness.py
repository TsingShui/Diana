"""Execute bounded DEX control/effect witnesses against explicitly authored stubs."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Callable, Protocol
from zipfile import BadZipFile, ZipFile
from zlib import error as DeflateError

from corpus_oracles import parameter_types, read_dex
from dex_calls import INVOKES, LOCATOR, WIDTHS, method_code, method_ref
from dex_truth import APK_FILE_BYTES, U16, U32
from evidence import file_digest

MAX_CODE_UNITS = 512
MAX_STEPS = 512
MAX_REGISTERS = 256
MAX_ARRAY_LENGTH = 256
MAX_HANDLERS = 64
MAX_SCENARIOS = 32
# Widths supplement the shared ordinary invoke reader, which rejects transfers.
BRANCHES = {0x28: 1, 0x29: 2, 0x2a: 3, **dict.fromkeys(range(0x32, 0x3e), 2)}
OTHER_WIDTHS = {0x1d: 1, 0x1e: 1, 0x27: 1, 0x26: 3, 0x2b: 3, 0x2c: 3,
                0xfa: 4, 0xfb: 4, 0xfc: 3, 0xfd: 3}
RESULTS = {0x0a, 0x0b, 0x0c}
COMPARISONS = {
    0x32: "eq", 0x33: "ne", 0x34: "lt", 0x35: "ge", 0x36: "gt", 0x37: "le",
    0x38: "eq", 0x39: "ne", 0x3a: "lt", 0x3b: "ge", 0x3c: "gt", 0x3d: "le",
}
COMPARE = {
    "eq": lambda x, y: x == y,
    "ne": lambda x, y: x != y,
    "lt": lambda x, y: x < y,
    "ge": lambda x, y: x >= y,
    "gt": lambda x, y: x > y,
    "le": lambda x, y: x <= y,
}
MISSING = object()


class Unavailable(ValueError):
    """A witness cannot establish the requested behavior within its model."""


@dataclass(frozen=True)
class Ref:
    name: str
    descriptor: str


@dataclass(frozen=True)
class Fault:
    name: str
    descriptor: str
    supertypes: tuple[str, ...]


class Raised(Exception):
    """A stub or the selected DEX method threw a modeled exception value."""

    def __init__(self, fault: Fault):
        super().__init__(fault.name)
        self.fault = fault


Value = int | str | Ref | Fault
Member = tuple[str, str, str]


class Model(Protocol):
    events: list[str]

    def static_field(self, member: Member) -> Value: ...
    def allocate(self, owner: str) -> Ref: ...
    def call(self, member: Member, receiver: Ref | None, arguments: tuple[Value, ...]) -> Value | None: ...
    def finish(self, value: Value | None, fault: Fault | None) -> None: ...


@dataclass(frozen=True)
class Scenario:
    id: str
    arguments: tuple[Value, ...]
    model: Callable[[], Model]
    expected: tuple[str, ...]


@dataclass(frozen=True)
class Instruction:
    pc: int
    word: int
    operands: tuple[int, ...]
    target: int | None = None

    @property
    def opcode(self):
        return self.word & 255

    @property
    def next(self):
        return self.pc + 1 + len(self.operands)


@dataclass(frozen=True)
class Protection:
    start: int
    end: int
    clauses: tuple[tuple[str | None, int], ...]


@dataclass(frozen=True)
class Program:
    dex: object
    registers: int
    inputs: tuple[str, ...]
    returns: str
    instructions: dict[int, Instruction]
    protection: tuple[Protection, ...]


def signed(value, bits):
    return value - (1 << bits) if value >> (bits - 1) else value


def sleb(dex, offset):
    value = 0
    for index in range(5):
        dex.data_range(offset, 1, "signed catch handler count")
        byte = dex.data[offset]
        offset += 1
        value |= (byte & 127) << (index * 7)
        if byte & 128 == 0:
            value = signed(value, (index + 1) * 7)
            if -(1 << 31) <= value < 1 << 31:
                return value, offset
            break
    raise ValueError("malformed signed catch handler count")


def decode_instructions(dex, start, count):
    words = [U16.unpack_from(dex.data, start + index * 2)[0] for index in range(count)]
    decoded = {}
    pc = 0
    while pc < count:
        word = words[pc]
        opcode = word & 255
        width = WIDTHS.get(opcode) or BRANCHES.get(opcode) or OTHER_WIDTHS.get(opcode)
        if (width is None or pc + width > count or opcode == 0 and word
                or opcode in {0x26, 0x2b, 0x2c}):
            raise Unavailable(f"unsupported or incomplete instruction layout at code unit {pc}")
        operands = tuple(words[pc + 1:pc + width])
        target = None
        if opcode == 0x28:
            target = pc + signed(word >> 8, 8)
        elif opcode == 0x29 or opcode in COMPARISONS:
            target = pc + signed(operands[0], 16)
        elif opcode == 0x2a:
            target = pc + signed(operands[0] | operands[1] << 16, 32)
        decoded[pc] = Instruction(pc, word, operands, target)
        pc += width
    for instruction in decoded.values():
        target = instruction.target
        if target is not None and (target not in decoded or decoded[target].opcode in RESULTS | {0x0d}):
            raise ValueError(f"invalid branch target at code unit {instruction.pc}")
    return decoded


def exception_table(dex, end, units, tries, decoded):
    if not tries:
        return ()
    if tries > MAX_HANDLERS:
        raise Unavailable("selected DEX method exceeds the try-region bound")
    start = end + (2 if units % 2 else 0)
    dex.data_range(end, start - end + 8 * tries, "try item table")
    if start != end and U16.unpack_from(dex.data, end)[0]:
        raise ValueError("DEX try item padding is nonzero")
    handler_base = cursor = start + 8 * tries
    handlers, cursor = dex.uleb(cursor)
    if not 0 < handlers <= MAX_HANDLERS:
        raise Unavailable("selected DEX method exceeds the catch-handler bound")
    mappings = {}
    for _ in range(handlers):
        offset = cursor - handler_base
        length, cursor = sleb(dex, cursor)
        if abs(length) > MAX_HANDLERS:
            raise Unavailable("selected DEX method exceeds the catch-clause bound")
        clauses = []
        types = set()
        for _ in range(abs(length)):
            index, cursor = dex.uleb(cursor)
            address, cursor = dex.uleb(cursor)
            descriptor = dex.descriptor(index)
            if not descriptor.startswith("L") or descriptor in types:
                raise ValueError("invalid or repeated catch type")
            types.add(descriptor)
            clauses.append((descriptor, address))
        if length <= 0:
            address, cursor = dex.uleb(cursor)
            clauses.append((None, address))
        if any(address not in decoded or decoded[address].opcode != 0x0d for _, address in clauses):
            raise Unavailable("catch entry must establish its exception register")
        mappings[offset] = tuple(clauses)
    result = []
    previous = 0
    for index in range(tries):
        position = start + index * 8
        first = U32.unpack_from(dex.data, position)[0]
        length = U16.unpack_from(dex.data, position + 4)[0]
        handle = U16.unpack_from(dex.data, position + 6)[0]
        last = first + length
        if (first < previous or not length or first not in decoded
                or (last != units and last not in decoded) or handle not in mappings):
            raise ValueError("invalid DEX try range or handler offset")
        previous = last
        result.append(Protection(first, last, mappings[handle]))
    return tuple(result)


def static_definition(dex, owner, method):
    row = dex.classes.offset(owner)
    start = U32.unpack_from(dex.data, row + 24)[0]
    counts = []
    for _ in range(4):
        count, start = dex.uleb(start)
        counts.append(count)
    # method_code already validates and bounds this encoded definition directory.
    for _ in range(2 * sum(counts[:2])):
        _, start = dex.uleb(start)
    for count in counts[2:]:
        index = 0
        for _ in range(count):
            delta, start = dex.uleb(start)
            index += delta
            access, start = dex.uleb(start)
            _, start = dex.uleb(start)
            if index == method:
                return bool(access & 0x0008)
    return False


def program(artifact, input_sha256, at, member):
    if artifact.stat().st_size > APK_FILE_BYTES or file_digest(artifact) != input_sha256:
        raise ValueError("APK differs from the pinned raw witness input")
    locator = LOCATOR.fullmatch(at)
    if locator is None:
        raise Unavailable("raw witness requires an original APK DEX method locator")
    entry, owner, index = map(int, locator.groups())
    with ZipFile(artifact) as archive:
        dex = read_dex(archive, entry)
    code = method_code(dex, owner, index, member)
    if code % 4:
        raise ValueError("selected DEX code item is unaligned")
    dex.data_range(code, 16, "raw witness code item")
    registers, inputs, _, tries = (U16.unpack_from(dex.data, code + offset)[0] for offset in (0, 2, 4, 6))
    units = U32.unpack_from(dex.data, code + 12)[0]
    if not 0 < units <= MAX_CODE_UNITS:
        raise Unavailable("selected DEX method exceeds the instruction bound")
    signature = member.split("->", 1)[1].split("(", 1)[1]
    types = tuple(parameter_types("(" + signature))
    if (not static_definition(dex, owner, index) or inputs != len(types) or registers < inputs
            or registers > MAX_REGISTERS or any(kind in {"J", "D", "F"} for kind in types)):
        raise Unavailable("raw witness needs a bounded static method with narrow incoming values")
    returns = signature.split(")", 1)[1]
    if returns in {"J", "D", "F"}:
        raise Unavailable("raw witness does not model this return type")
    start = code + 16
    dex.data_range(start, units * 2, "raw witness instructions")
    decoded = decode_instructions(dex, start, units)
    protection = exception_table(dex, start + units * 2, units, tries, decoded)
    return Program(dex, registers, types, returns, decoded, protection)


def check_value(value, descriptor):
    if descriptor == "V":
        return value is None
    if descriptor.startswith(("L", "[")):
        if type(value) is int:
            return value == 0
        if descriptor == "Ljava/lang/Object;":
            return isinstance(value, (Ref, Fault, str))
        if isinstance(value, Ref):
            return value.descriptor == descriptor
        if isinstance(value, Fault):
            return descriptor in (value.descriptor, *value.supertypes)
        return isinstance(value, str) and descriptor == "Ljava/lang/String;"
    if type(value) is not int:
        return False
    limits = {"Z": (0, 2), "B": (-128, 128), "S": (-(1 << 15), 1 << 15),
              "C": (0, 1 << 16), "I": (-(1 << 31), 1 << 31)}
    return descriptor in limits and limits[descriptor][0] <= value < limits[descriptor][1]


def register_words(instruction):
    word = instruction.word
    trailing = instruction.operands[1]
    if instruction.opcode == 0x25 or instruction.opcode >= 0x74:
        return tuple(range(trailing, trailing + (word >> 8)))
    count = word >> 12
    if count > 5:
        raise Unavailable(f"invalid invoke register count at code unit {instruction.pc}")
    return (*((trailing >> (4 * index)) & 15 for index in range(4)), (word >> 8) & 15)[:count]


def static_field(dex, index):
    row = dex.fields.offset(index)
    owner = dex.descriptor(U16.unpack_from(dex.data, row)[0])
    descriptor = dex.descriptor(U16.unpack_from(dex.data, row + 2)[0])
    name = dex.string(U32.unpack_from(dex.data, row + 4)[0])
    return owner, name, descriptor


def show_value(item):
    if isinstance(item, (Ref, Fault)):
        return {"identity": item.name, "type": item.descriptor}
    return item


class Execution:
    def __init__(self, selected: Program, scenario: Scenario):
        self.program = selected
        self.model = scenario.model()
        self.frames: list = [MISSING] * selected.registers
        self.trace = []
        self.pending: tuple[str, Value] | None = None
        self.outcome: tuple[Value | None, Fault | None] | None = None
        self.caught: Fault | None = None
        self.uninitialized: set[Ref] = set()
        self.array_lengths: dict[Ref, int] = {}
        self.allocated = {value for value in scenario.arguments if isinstance(value, Ref)}
        if len(scenario.arguments) != len(selected.inputs):
            raise Unavailable("scenario arguments differ from the incoming DEX words")
        for number, (argument, descriptor) in enumerate(zip(scenario.arguments, selected.inputs),
                                                       selected.registers - len(selected.inputs)):
            if not check_value(argument, descriptor):
                raise Unavailable("scenario argument is outside the supported DEX prototype")
            self.frames[number] = argument

    def read(self, register):
        if not 0 <= register < len(self.frames) or self.frames[register] is MISSING:
            raise Unavailable(f"register v{register} has no established value")
        return self.frames[register]

    def store(self, register, value):
        if not 0 <= register < len(self.frames):
            raise Unavailable(f"register v{register} is out of range")
        self.frames[register] = value

    def move(self, instruction):
        word, opcode = instruction.word, instruction.opcode
        family = (opcode - 1) % 3
        if family == 0:
            dest, source = (word >> 8) & 15, word >> 12
        elif family == 1:
            dest, source = word >> 8, instruction.operands[0]
        else:
            dest, source = instruction.operands
        value = self.read(source)
        if opcode <= 0x03 and type(value) is not int:
            raise Unavailable("non-object move has no modeled integer input")
        if opcode >= 0x07 and type(value) is int and value != 0:
            raise Unavailable("object move has no modeled reference input")
        self.store(dest, value)

    def invoke(self, instruction):
        target = method_ref(self.program.dex, instruction.operands[0])
        signature = target[2]
        arguments = parameter_types(signature)
        returns = signature.split(")", 1)[1]
        words = register_words(instruction)
        static = instruction.opcode in {0x71, 0x77}
        if len(words) != len(arguments) + int(not static):
            raise Unavailable("invoke requires a different number of narrow register words")
        receiver = None if static else self.read(words[0])
        if not static and not isinstance(receiver, Ref):
            raise Unavailable("invoke does not have an established modeled receiver")
        constructor = target[1] == "<init>"
        direct = instruction.opcode in {0x70, 0x76}
        if constructor:
            if not direct or receiver not in self.uninitialized or returns != "V":
                raise Unavailable("constructor receiver is not the matching newly allocated object")
        elif direct or instruction.opcode in {0x6f, 0x75} or receiver in self.uninitialized:
            raise Unavailable("special dispatch is outside the modeled invocation")
        if receiver is not None and receiver.descriptor != target[0]:
            raise Unavailable("receiver nominal type differs from the selected DEX owner")
        values = tuple(self.read(register) for register in words[int(not static):])
        if any(not check_value(value, kind) or value in self.uninitialized
               for value, kind in zip(values, arguments)):
            raise Unavailable("invoke argument is outside the modeled prototype")
        point = {"pc": instruction.pc, "op": "invoke", "member": list(target),
                 "receiver": show_value(receiver), "arguments": [show_value(value) for value in values]}
        self.trace.append(point)
        result = self.model.call(target, receiver, values)
        if not check_value(result, returns):
            raise Unavailable("stub result is outside the original DEX prototype")
        if constructor:
            self.uninitialized.remove(receiver)
        self.pending = None if returns == "V" else (returns, result)
        point["value"] = show_value(result)

    def allocate_array(self, pc, descriptor, length):
        if (type(length) is not int or not 0 <= length <= MAX_ARRAY_LENGTH
                or not descriptor.startswith("[")
                or not (descriptor[1:] == "I" or descriptor[1:].startswith(("L", "[")))):
            raise Unavailable("array shape is outside the bounded witness")
        allocate = getattr(self.model, "allocate_array", None)
        if not callable(allocate):
            raise Unavailable("the scenario has no array allocation model")
        point = {"pc": pc, "op": "allocate-array", "descriptor": descriptor, "length": length}
        self.trace.append(point)
        array = allocate(descriptor, length)
        if not isinstance(array, Ref) or array.descriptor != descriptor or array in self.allocated:
            raise Unavailable("stub allocation does not establish a unique modeled array")
        self.allocated.add(array)
        self.array_lengths[array] = length
        point["value"] = show_value(array)
        return array

    def store_array(self, pc, array, index, value):
        length = self.array_lengths.get(array)
        if (length is None or type(index) is not int or not 0 <= index < length
                or not check_value(value, array.descriptor[1:]) or value in self.uninitialized):
            raise Unavailable("array write is outside the bounded witness")
        store = getattr(self.model, "store_array", None)
        if not callable(store):
            raise Unavailable("the scenario has no array store model")
        self.trace.append({"pc": pc, "op": "write-array", "array": show_value(array),
                           "index": index, "value": show_value(value)})
        store(array, index, value)

    def array_instruction(self, instruction):
        pc, word, opcode = instruction.pc, instruction.word, instruction.opcode
        if opcode in (0x4b, 0x4d):
            operands = instruction.operands[0]
            array, index = self.read(operands & 255), self.read(operands >> 8)
            if not isinstance(array, Ref) or ((opcode == 0x4b) != (array.descriptor == "[I")):
                raise Unavailable("array write opcode disagrees with the modeled component type")
            self.store_array(pc, array, index, self.read(word >> 8))
            return
        descriptor = self.program.dex.descriptor(instruction.operands[0])
        if opcode == 0x23:
            array = self.allocate_array(pc, descriptor, self.read(word >> 12))
            self.store((word >> 8) & 15, array)
            return
        values = tuple(self.read(register) for register in register_words(instruction))
        if any(not check_value(value, descriptor[1:]) or value in self.uninitialized for value in values):
            raise Unavailable("filled array operand is outside the modeled component type")
        array = self.allocate_array(pc, descriptor, len(values))
        for index, value in enumerate(values):
            self.store_array(pc, array, index, value)
        self.pending = descriptor, array

    def route(self, pc, fault):
        self.pending = None
        if ("Ljava/lang/Throwable;" not in (fault.descriptor, *fault.supertypes)
                or "Ljava/lang/Object;" not in fault.supertypes):
            raise Unavailable("modeled exception ancestry is incomplete")
        for region in self.program.protection:
            if not region.start <= pc < region.end:
                continue
            for descriptor, address in region.clauses:
                if descriptor is None or descriptor in (fault.descriptor, *fault.supertypes):
                    self.trace.append({"pc": pc, "op": "catch", "fault": show_value(fault),
                                       "target": address, "clause": descriptor})
                    self.caught = fault
                    return address
            break
        raise Raised(fault)

    def transfer(self, instruction):
        pc, opcode, word = instruction.pc, instruction.opcode, instruction.word
        if opcode in (0x28, 0x29, 0x2a):
            target = instruction.target
        else:
            if opcode < 0x38:
                left = self.read((word >> 8) & 15)
                right = self.read(word >> 12)
                if type(left) is not int or type(right) is not int:
                    raise Unavailable("two-register comparison requires modeled integer operands")
            else:
                left, right = self.read(word >> 8), 0
                if type(left) is not int:
                    if opcode not in (0x38, 0x39) or left in self.uninitialized:
                        raise Unavailable("non-integer zero comparison is outside the effect model")
                    left = 1  # Modeled references and strings are non-null.
            target = instruction.target if COMPARE[COMPARISONS[opcode]](left, right) else instruction.next
        self.trace.append({"pc": pc, "op": "branch", "target": target})
        return target

    def finish(self, pc, value=None, fault=None):
        self.outcome = value, fault
        if fault is None:
            self.trace.append({"pc": pc, "op": "return", "value": show_value(value)})
        else:
            self.trace.append({"pc": pc, "op": "escape", "fault": show_value(fault)})
        return self.trace

    def run(self):
        pc = 0
        for _ in range(MAX_STEPS):
            instruction = self.program.instructions.get(pc)
            if instruction is None:
                raise Unavailable("selected path runs outside the original instructions")
            next_pc = instruction.next
            opcode, word, operands = instruction.opcode, instruction.word, instruction.operands
            if opcode not in RESULTS:
                self.pending = None
            try:
                if opcode in (0x01, 0x02, 0x03, 0x07, 0x08, 0x09):
                    self.move(instruction)
                elif opcode in (0x0a, 0x0c):
                    if self.pending is None:
                        raise Unavailable("move-result has no immediately preceding result producer")
                    kind, result = self.pending
                    if (opcode == 0x0c) != kind.startswith(("L", "[")):
                        raise Unavailable("move-result form disagrees with the original result type")
                    self.store(word >> 8, result)
                    self.pending = None
                elif opcode == 0x0d:
                    if self.caught is None:
                        raise Unavailable("move-exception is outside an established handler entry")
                    self.store(word >> 8, self.caught)
                    self.caught = None
                elif opcode in (0x0e, 0x0f, 0x11):
                    value = None if opcode == 0x0e else self.read(word >> 8)
                    if ((opcode == 0x11) != self.program.returns.startswith(("L", "["))
                            or value in self.uninitialized or not check_value(value, self.program.returns)):
                        raise Unavailable("return disagrees with the selected DEX prototype")
                    return self.finish(pc, value)
                elif opcode == 0x12:
                    self.store((word >> 8) & 15, signed(word >> 12, 4))
                elif opcode == 0x13:
                    self.store(word >> 8, signed(operands[0], 16))
                elif opcode in (0x1a, 0x1b):
                    index = operands[0] if opcode == 0x1a else operands[0] | operands[1] << 16
                    self.store(word >> 8, self.program.dex.string(index))
                elif opcode == 0x22:
                    owner = self.program.dex.descriptor(operands[0])
                    value = self.model.allocate(owner)
                    if not isinstance(value, Ref) or value.descriptor != owner or value in self.allocated:
                        raise Unavailable("stub allocation does not establish a unique modeled object")
                    self.allocated.add(value)
                    self.uninitialized.add(value)
                    self.store(word >> 8, value)
                    self.trace.append({"pc": pc, "op": "allocate", "value": show_value(value)})
                elif opcode in (0x23, 0x24, 0x25, 0x4b, 0x4d):
                    self.array_instruction(instruction)
                elif opcode == 0x27:
                    value = self.read(word >> 8)
                    if not isinstance(value, Fault):
                        raise Unavailable("throw does not have an established modeled exception")
                    self.trace.append({"pc": pc, "op": "throw", "fault": show_value(value)})
                    raise Raised(value)
                elif opcode in BRANCHES:
                    next_pc = self.transfer(instruction)
                elif opcode == 0x62:
                    target = static_field(self.program.dex, operands[0])
                    value = self.model.static_field(target)
                    if not check_value(value, target[2]) or value in self.uninitialized:
                        raise Unavailable("stub field value is outside the original field descriptor")
                    self.store(word >> 8, value)
                    self.trace.append({"pc": pc, "op": "read-static", "member": list(target), "value": show_value(value)})
                elif opcode in INVOKES:
                    self.invoke(instruction)
                elif opcode:
                    raise Unavailable(f"unmodeled DEX opcode 0x{opcode:02x} at code unit {pc}")
            except Raised as error:
                try:
                    next_pc = self.route(pc, error.fault)
                except Raised:
                    return self.finish(pc, fault=error.fault)
            pc = next_pc
        raise Unavailable("selected raw DEX path reached the step bound")


def check(scenario, status, *, detected=None, trace=None, detail=None):
    return {"id": scenario.id, "status": status, "expected": list(scenario.expected),
            "observed": detected, "trace": trace or [], **({"detail": detail} if detail else {})}


def evaluate(artifact: Path, *, input_sha256: str, at: str, member: str,
             scenarios: list[Scenario]) -> list[dict]:
    """Read and execute the original retained DEX method for authored scenarios.

    The guest runs no Android code. Stub identities and effects are fixture
    hypotheses and every unmodeled reached operation abstains.
    """
    if (not 0 < len(scenarios) <= MAX_SCENARIOS
            or len({scenario.id for scenario in scenarios}) != len(scenarios)):
        raise ValueError("raw witness needs a bounded set of unique scenarios")
    try:
        selected = program(artifact, input_sha256, at, member)
    except Unavailable as error:
        return [check(scenario, "UNCHECKED", detail=str(error)) for scenario in scenarios]
    except (OSError, ValueError, BadZipFile, DeflateError, EOFError) as error:
        return [check(scenario, "SETUP_ERROR", detail=str(error)) for scenario in scenarios]

    results = []
    for scenario in scenarios:
        execution = None
        try:
            execution = Execution(selected, scenario)
            trace = execution.run()
            execution.model.finish(*execution.outcome)
        except Unavailable as error:
            results.append(check(scenario, "UNCHECKED", trace=execution.trace if execution else None, detail=str(error)))
        except (ValueError, KeyError, TypeError, Raised) as error:
            results.append(check(scenario, "SETUP_ERROR", trace=execution.trace if execution else None, detail=str(error)))
        else:
            events = list(execution.model.events)
            matches = events == list(scenario.expected)
            results.append(check(scenario, "PASS" if matches else "SETUP_ERROR", detected=events, trace=trace,
                                 detail=None if matches else "raw DEX control disagrees with the authored scenario"))
    try:
        if file_digest(artifact) != input_sha256:
            raise ValueError("APK changed while executing the raw witness")
    except (OSError, ValueError) as error:
        return [check(scenario, "SETUP_ERROR", detail=str(error)) for scenario in scenarios]
    return results
