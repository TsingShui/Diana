"""Emit bounded, standalone JVM comparison bytecode with no assembler dependency."""
from argparse import ArgumentParser
from pathlib import Path
from struct import pack


def u16(*values):
    return pack(">" + "H" * len(values), *values)


def u32(value):
    return pack(">I", value)


pool = []


def constant(tag, payload):
    pool.append(bytes([tag]) + payload)
    return len(pool)


def utf8(text):
    units = text.encode()
    return constant(1, u16(len(units)) + units)


def class_ref(owner):
    return constant(7, u16(utf8(owner)))


def member_ref(tag, owner, name, desc):
    signature = constant(12, u16(utf8(name), utf8(desc)))
    return constant(tag, u16(owner, signature))


def op(opcode, reference):
    return bytes([opcode]) + u16(reference)


klass = class_ref("fixture/FloatCompare")
parent = class_ref("java/lang/Object")
trace = member_ref(9, klass, "trace", "I")
left = member_ref(10, klass, "left", "()F")
right = member_ref(10, klass, "right", "()F")
left_nan = member_ref(10, klass, "leftNaN", "()F")
error_signature = "(Ljava/lang/RuntimeException;)"
right_maybe = member_ref(10, klass, "rightMaybe", error_signature + "F")
nan = constant(4, u32(0x7FC00000))
field = u16(0x0009, utf8("trace"), utf8("I"), 0)
code_name = utf8("Code")


def method(name, desc, code, stack, locals_=0):
    payload = u16(stack, locals_) + u32(len(code)) + code + u16(0, 0)
    return (
        u16(0x0009, utf8(name), utf8(desc), 1, code_name)
        + u32(len(payload))
        + payload
    )


def mark(digit):
    return (
        op(0xB2, trace) + bytes([0x10, 10, 0x68, 0x03 + digit, 0x60])
        + op(0xB3, trace)
    )


# fcmpg, iconst_1, if_icmpne +5, iconst_1; ireturn; iconst_0; ireturn.
against_one = bytes.fromhex("9604a0000504ac03ac")
# fcmpg, ifle +5, iconst_1; ireturn; iconst_0; ireturn.
against_zero = bytes.fromhex("969e000504ac03ac")
both = op(0xB8, left) + op(0xB8, right)
both_nan = op(0xB8, left_nan) + bytes([0x2A]) + op(0xB8, right_maybe)
methods = [
    method("left", "()F", mark(1) + bytes.fromhex("0dae"), 2),
    method("right", "()F", mark(2) + bytes.fromhex("0cae"), 2),
    method("leftNaN", "()F", mark(1) + bytes([0x12, nan, 0xAE]), 2),
    method("rightMaybe", error_signature + "F", mark(2) + bytes.fromhex("2ac600052abf0cae"), 2, 1),
    method("one", "()Z", both + against_one, 2),
    method("zero", "()Z", both + against_zero, 2),
    method("value", "()I", both + bytes.fromhex("96ac"), 2),
    method("nanSkips", error_signature + "Z", both_nan + against_one, 2, 1),
]


def predicate(test):
    return test + bytes.fromhex("a0000504ac03ac")


def extend():
    """Keep the base methods and add independently chosen comparison operands."""
    lhs_f = member_ref(10, klass, "floatLeft", "(F)F")
    rhs_f = member_ref(10, klass, "floatRight", "(F)F")
    lhs_d = member_ref(10, klass, "doubleLeft", "(D)D")
    rhs_d = member_ref(10, klass, "doubleRight", "(D)D")
    f_prefix = bytes([0x22]) + op(0xB8, lhs_f) + bytes([0x23]) + op(0xB8, rhs_f)
    d_prefix = bytes([0x26]) + op(0xB8, lhs_d) + bytes([0x28]) + op(0xB8, rhs_d)
    negated = (bytes([0x22]) + op(0xB8, lhs_f) + bytes([0x76, 0x23])
               + op(0xB8, rhs_f) + bytes([0x76]))
    methods.extend([
        method("floatLeft", "(F)F", mark(1) + bytes.fromhex("22ae"), 2, 1),
        method("floatRight", "(F)F", mark(2) + bytes.fromhex("22ae"), 2, 1),
        method("doubleLeft", "(D)D", mark(1) + bytes.fromhex("26af"), 2, 2),
        method("doubleRight", "(D)D", mark(2) + bytes.fromhex("26af"), 2, 2),
        method("floatHigh", "(FF)I", f_prefix + bytes.fromhex("96ac"), 2, 2),
        method("floatLow", "(FF)I", f_prefix + bytes.fromhex("95ac"), 2, 2),
        method("doubleHigh", "(DD)I", d_prefix + bytes.fromhex("98ac"), 4, 4),
        method("doubleLow", "(DD)I", d_prefix + bytes.fromhex("97ac"), 4, 4),
        method("highPositive", "(FF)Z", f_prefix + predicate(bytes.fromhex("9604")), 2, 2),
        method("lowNegative", "(FF)Z", f_prefix + predicate(bytes.fromhex("9502")), 2, 2),
        method("nestedOperands", "(FF)Z", negated + predicate(bytes.fromhex("9604")), 2, 2),
        # (fcmpl(a, b) + 1) == 2. The compare is inside a numeric predicate operand.
        method("nestedResult", "(FF)Z", f_prefix + predicate(bytes.fromhex("95046005")), 2, 2),
    ])


def main():
    options = ArgumentParser(description=__doc__)
    options.add_argument("output", type=Path)
    options.add_argument("--base-only", action="store_true")
    args = options.parse_args()
    if not args.base_only:
        extend()
    bytecode = (
        bytes.fromhex("cafebabe") + u16(0, 49, len(pool) + 1) + b"".join(pool)
        + u16(0x0031, klass, parent, 0, 1) + field + u16(len(methods))
        + b"".join(methods) + u16(0)
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(bytecode)


if __name__ == "__main__":
    main()
