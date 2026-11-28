"""Replay only the instruction subset of the two authored standalone DEX fixtures."""

from argparse import ArgumentParser
import json
from pathlib import Path

from dex_calls import method_code, method_ref
from dex_truth import Dex, U16, U32
from dex_witness import decode_instructions, register_words, signed, static_definition, static_field


ROOT = Path(__file__).resolve().parent
EFFECTS = "Lfixture/BooleanEffects;"
FAILURE = (EFFECTS, "failure", "Ljava/lang/IllegalStateException;")
FLAGS = (EFFECTS, "flags", "I")
THROW_AT = (EFFECTS, "throwAt", "I")
TRACE = (EFFECTS, "trace", "I")


class FixtureFault(Exception):
    pass


def load(path):
    with path.open("rb") as stream:
        data = stream.read(65537)
    if len(data) > 65536:
        raise ValueError("standalone fixture exceeds the replay size limit")
    dex = Dex(data)
    bodies = {}
    for owner in range(dex.classes.count):
        row = dex.classes.offset(owner)
        owner_index = U32.unpack_from(dex.data, row)[0]
        definitions = U32.unpack_from(dex.data, row + 24)[0]
        methods, _ = dex.class_methods(owner_index, definitions)
        for index, member in methods:
            target = method_ref(dex, index)
            assert target[1] != "<clinit>" and static_definition(dex, owner, index)
            code = method_code(dex, owner, index, member)
            dex.data_range(code, 16, "fixture code header")
            registers, inputs, _, tries = [U16.unpack_from(dex.data, code + offset)[0]
                                           for offset in (0, 2, 4, 6)]
            units = U32.unpack_from(dex.data, code + 12)[0]
            assert code % 4 == 0 and not tries
            assert 0 <= inputs <= registers <= 8 and 0 < units <= 32
            dex.data_range(code + 16, 2 * units, "fixture code units")
            bodies[target] = registers, inputs, decode_instructions(dex, code + 16, units)
    return dex, bodies


def execute(dex, bodies, target, fields, args=(), depth=0):
    registers, inputs, instructions = bodies[target]
    assert depth <= 2 and len(args) == inputs
    values = {register: value for register, value in enumerate(args, registers - inputs)}
    unavailable = object()
    pending = unavailable
    pc = 0
    for _ in range(32):
        instruction = instructions[pc]
        word, opcode, operands = instruction.word, instruction.opcode, instruction.operands
        operand = word >> 8
        a, b = operand & 15, operand >> 4
        following = instruction.next
        if opcode == 0x0A:
            assert type(pending) is int
            values[operand], pending = pending, unavailable
        else:
            pending = unavailable
            if opcode == 0x12:
                values[a] = signed(b, 4)
            elif opcode in (0x38, 0x39):
                if (values[operand] == 0) == (opcode == 0x38):
                    following = instruction.target
            elif opcode == 0x33:
                if values[a] != values[b]:
                    following = instruction.target
            elif opcode == 0x28:
                following = instruction.target
            elif opcode == 0x71:
                callee = method_ref(dex, operands[0])
                assert callee[0] == EFFECTS
                parameters = tuple(values[register] for register in register_words(instruction))
                pending = execute(dex, bodies, callee, fields, parameters, depth + 1)
            elif opcode == 0x0F:
                result = values[operand]
                assert target[2].endswith(")Z") and result in (0, 1)
                return result
            elif opcode == 0x0E:
                assert target[2].endswith(")V")
                return None
            elif opcode in (0x60, 0x62, 0x67):
                field = static_field(dex, operands[0])
                if opcode == 0x67:
                    assert field == TRACE
                    fields[field] = values[operand]
                else:
                    assert (field == FAILURE) == (opcode == 0x62)
                    values[operand] = fields[field]
            elif opcode == 0x27:
                fault = values[operand]
                assert isinstance(fault, FixtureFault)
                raise fault
            elif opcode in (0xD8, 0xDA, 0xDD):
                source = values[operands[0] & 255]
                literal = signed(operands[0] >> 8, 8)
                if opcode == 0xD8:
                    values[operand] = signed((source + literal) & 0xFFFFFFFF, 32)
                elif opcode == 0xDA:
                    values[operand] = signed((source * literal) & 0xFFFFFFFF, 32)
                else:
                    values[operand] = source & literal
            elif opcode == 0xB0:
                values[a] = signed((values[a] + values[b]) & 0xFFFFFFFF, 32)
            elif opcode == 0xBA:
                values[a] = signed((values[a] & 0xFFFFFFFF) >> (values[b] & 31), 32)
            else:
                raise AssertionError(f"unexpected fixture opcode 0x{opcode:02x}")
        pc = following
    raise AssertionError("fixture exceeded step bound")


def main():
    parser = ArgumentParser(description=__doc__)
    parser.add_argument("fixture", nargs="?", default=ROOT, type=Path)
    options = parser.parse_args()
    fixture = options.fixture
    manifest = json.loads((fixture / "decbench.json").read_text(encoding="utf-8"))
    scenarios = json.loads((fixture / "scenarios.json").read_text(encoding="utf-8"))
    program = load(fixture / manifest["input"])
    dex, _ = program
    for spec in manifest["methods"]:
        _, _, owner, index = spec["at"].split(":")
        target = method_ref(dex, int(index[1:]))
        method_code(dex, int(owner[1:]), int(index[1:]), spec["member"])
        for scenario in scenarios[target[1]]:
            flag, throws = scenario["arguments"][-2:]
            supplied = FixtureFault(scenario["id"])
            fields = {FLAGS: int(flag), THROW_AT: int(throws), TRACE: 0, FAILURE: supplied}
            try:
                actual_result = "true" if execute(*program, target, fields) else "false"
            except FixtureFault as observed:
                actual_result = "same-fault" if observed is supplied else "different-fault"
            actual = [actual_result, str(fields[TRACE])]
            if actual != scenario["expected"]:
                raise ValueError(f"{scenario['id']} differs: {actual!r} != {scenario['expected']!r}")
            print(scenario["id"], *actual, sep="\t")


if __name__ == "__main__":
    main()
