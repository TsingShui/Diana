"""Emit JVM NaN constants with exactly the authored IEEE 754 bit patterns."""

from argparse import ArgumentParser
from pathlib import Path
from struct import pack


def u16(*values):
    return pack(">" + "H" * len(values), *values)


def u32(value):
    return pack(">I", value)


class ConstantPool:
    def __init__(self):
        self.entries = []

    def add(self, tag, payload):
        self.entries.append(bytes([tag]) + payload)
        index = len(self.entries)
        if tag == 6:
            self.entries.append(b"")
        return index

    def utf8(self, value):
        text = value.encode("ascii")
        return self.add(1, u16(len(text)) + text)

    def class_ref(self, name):
        return self.add(7, u16(self.utf8(name)))

    def method_ref(self, owner, name, descriptor):
        signature = self.add(12, u16(self.utf8(name), self.utf8(descriptor)))
        return self.add(10, u16(self.class_ref(owner), signature))

    def attribute(self, name, value):
        return u16(self.utf8(name)) + u32(len(value)) + value

    def field(self, name, descriptor, index):
        return u16(0x0019, self.utf8(name), self.utf8(descriptor), 1) + self.attribute(
            "ConstantValue", u16(index)
        )

    def method(self, name, descriptor, code, stack, locals_=0):
        body = u16(stack, locals_) + u32(len(code)) + code + u16(0, 0)
        return u16(0x0009, self.utf8(name), self.utf8(descriptor), 1) + self.attribute("Code", body)

    def file(self, name, parent, fields, methods):
        # Version 49 allows branch code without a StackMapTable.
        owner = self.class_ref(name)
        super_class = self.class_ref(parent)
        return (bytes.fromhex("cafebabe") + u16(0, 49, len(self.entries) + 1)
                + b"".join(self.entries) + u16(0x0031, owner, super_class, 0, len(fields))
                + b"".join(fields) + u16(len(methods)) + b"".join(methods) + u16(0))


def nan_methods():
    pool = ConstantPool()
    fa = pool.add(4, u32(0x7FC01234))
    fb = pool.add(4, u32(0xFFC05678))
    da = pool.add(6, pack(">Q", 0xFFF8000000005678))
    db = pool.add(6, pack(">Q", 0x7FF8000000004321))
    zero = pool.add(4, u32(0x80000000))
    infinity = pool.add(6, pack(">Q", 0x7FF0000000000000))
    observe_f = pool.method_ref("java/lang/Float", "floatToRawIntBits", "(F)I")
    observe_d = pool.method_ref("java/lang/Double", "doubleToRawLongBits", "(D)J")
    # Shared observer after loading one of two input payloads. The branch and
    # goto target the other load and the shared observer respectively.
    select_f = (bytes.fromhex("1a99000812") + bytes([fa]) + bytes.fromhex("a7000512")
                + bytes([fb, 0xB8]) + u16(observe_f) + bytes([0xAC]))
    select_d = (bytes.fromhex("1a99000914") + u16(da) + bytes.fromhex("a7000614")
                + u16(db) + bytes([0xB8]) + u16(observe_d) + bytes([0xAD]))
    methods = [
        pool.method("floatNaN", "()F", bytes([0x12, fa, 0xAE]), 1),
        pool.method("doubleNaN", "()D", bytes([0x14]) + u16(da) + bytes([0xAF]), 2),
        pool.method("selectFloatBits", "(Z)I", select_f, 1, 1),
        pool.method("selectDoubleBits", "(Z)J", select_d, 2, 1),
        pool.method("negativeZero", "()F", bytes([0x12, zero, 0xAE]), 1),
        pool.method("positiveInfinity", "()D", bytes([0x14]) + u16(infinity) + bytes([0xAF]), 2),
    ]
    return pool.file("NaNWork", "java/lang/Object", [], methods)


def nan_fields():
    pool = ConstantPool()
    fields = [
        pool.field("F_NAN", "F", pool.add(4, u32(0x7FC01234))),
        pool.field("D_NAN", "D", pool.add(6, pack(">Q", 0xFFF8000000005678))),
        pool.field("F_ZERO", "F", pool.add(4, u32(0x80000000))),
        pool.field("D_INF", "D", pool.add(6, pack(">Q", 0x7FF0000000000000))),
    ]
    methods = [pool.method("activate", "()I", bytes.fromhex("06ac"), 1)]
    return pool.file("NaNChild", "ObservingParent", fields, methods)


def main():
    parser = ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    output = parser.parse_args().output
    output.mkdir(parents=True, exist_ok=True)
    (output / "NaNWork.class").write_bytes(nan_methods())
    (output / "NaNChild.class").write_bytes(nan_fields())


if __name__ == "__main__":
    main()
