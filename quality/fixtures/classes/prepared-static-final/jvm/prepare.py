"""Emit the original DefaultChild class from independently specified JVM facts."""

from argparse import ArgumentParser
from pathlib import Path
from struct import pack


def u16(*values):
    return pack(">" + "H" * len(values), *values)


def u32(value):
    return pack(">I", value)


def class_bytes():
    pool = []

    def constant(tag, payload):
        pool.append(bytes([tag]) + payload)
        return len(pool)

    def utf8(text):
        data = text.encode("ascii")
        return constant(1, u16(len(data)) + data)

    def attribute(name, data):
        return u16(utf8(name)) + u32(len(data)) + data

    def field(name, descriptor, encoded=None):
        attributes = b"" if encoded is None else attribute("ConstantValue", u16(encoded))
        return u16(0x0019, utf8(name), utf8(descriptor), int(bool(attributes))) + attributes

    child = constant(7, u16(utf8("DefaultChild")))
    superclass = constant(7, u16(utf8("ObservableParent")))
    constructor_name = utf8("<init>")
    constructor_type = utf8("()V")
    name_and_type = constant(12, u16(constructor_name, constructor_type))
    parent_constructor = constant(10, u16(superclass, name_and_type))
    zero = constant(3, u32(0))
    interned = constant(8, u16(utf8("interned")))
    fields = [
        field("PREPARED_ZERO", "I"),
        field("PREPARED_NULL", "Ljava/lang/String;"),
        field("PREPARED_BYTE", "B"),
        field("PREPARED_SHORT", "S"),
        field("COMPILED_ZERO", "I", zero),
        field("COMPILED_TEXT", "Ljava/lang/String;", interned),
    ]
    # aload_0; invokespecial ObservableParent.<init>(); return.
    code = b"\x2a\xb7" + u16(parent_constructor) + b"\xb1"
    constructor = u16(0x0001, constructor_name, constructor_type, 1) + attribute(
        "Code", u16(1, 1) + u32(len(code)) + code + u16(0, 0)
    )
    body = u16(0x0021, child, superclass, 0, len(fields)) + b"".join(fields)
    # One instance constructor, no <clinit> and no putstatic.
    return (bytes.fromhex("cafebabe") + u16(0, 61, len(pool) + 1) + b"".join(pool)
            + body + u16(1) + constructor + u16(0))


def main():
    parser = ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    output = parser.parse_args().output
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(class_bytes())


if __name__ == "__main__":
    main()
