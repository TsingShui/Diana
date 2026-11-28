"""Read and mutate only the authored fixture's JVM method attributes."""

from dataclasses import dataclass
from struct import pack


POOL_BYTES = {
    3: 4, 4: 4, 5: 8, 6: 8, 7: 2, 8: 2, 9: 4, 10: 4, 11: 4,
    12: 4, 15: 3, 16: 2, 17: 4, 18: 4, 19: 2, 20: 2,
}


class Reader:
    def __init__(self, data):
        self.data = data
        self.position = 0

    def read(self, size):
        start = self.position
        end = start + size
        if size < 0 or end > len(self.data):
            raise ValueError("truncated fixture class file")
        self.position = end
        return self.data[start:end]

    def number(self, size):
        return int.from_bytes(self.read(size), "big")


@dataclass(frozen=True)
class Default:
    ordinal: int
    descriptor: bytes
    count_at: int
    attribute_at: int
    value_at: int
    end: int


def constant_pool(reader):
    count = reader.number(2)
    strings = {}
    index = 1
    while index < count:
        tag = reader.number(1)
        if tag == 1:
            strings[index] = reader.read(reader.number(2))
        else:
            reader.read(POOL_BYTES[tag])
        index += 2 if tag in (5, 6) else 1
    return strings


def read_members(reader, strings):
    for ordinal in range(reader.number(2)):
        reader.read(2)  # access flags
        name = strings[reader.number(2)]
        descriptor = strings[reader.number(2)]
        count_at = reader.position
        for _ in range(reader.number(2)):
            start = reader.position
            attribute = strings[reader.number(2)]
            length = reader.number(4)
            value = reader.position
            reader.read(length)
            yield name, attribute, Default(ordinal, descriptor, count_at, start, value, reader.position)


def default_attributes(data):
    reader = Reader(data)
    if reader.read(4) != b"\xca\xfe\xba\xbe":
        raise ValueError("the fixture class header is missing")
    reader.read(4)  # minor, major
    strings = constant_pool(reader)
    reader.read(6)  # access flags, current class, superclass
    reader.read(reader.number(2) * 2)  # interfaces
    for _ in read_members(reader, strings):
        pass
    defaults = {}
    for name, attribute, entry in read_members(reader, strings):
        if attribute != b"AnnotationDefault":
            continue
        member = name.decode("ascii")
        if member in defaults:
            raise ValueError(f"original fixture has a duplicate default for {member}")
        defaults[member] = entry
    return defaults, strings


def class_literal_offset(data, entry, strings, descriptor):
    value = Reader(data[entry.value_at:entry.end])
    if value.read(1) != b"[":
        raise ValueError("expected an array of class literals")
    matches = []
    for _ in range(value.number(2)):
        position = entry.value_at + value.position
        if value.read(1) != b"c":
            raise ValueError("expected a class literal in the fixture array")
        if strings[value.number(2)] == descriptor:
            matches.append(position)
    if len(matches) != 1 or value.position != len(value.data):
        raise ValueError("original class literal identity is not unique")
    return matches[0]


def malformed_defaults(data, defaults):
    scalar = defaults["number"]
    if scalar.end - scalar.value_at != 3 or data[scalar.value_at] != ord("I"):
        raise ValueError("expected an integer default attribute")
    if data[scalar.count_at:scalar.attribute_at] != b"\x00\x01":
        raise ValueError("expected the number method to have one attribute")
    trailing = (data[:scalar.attribute_at + 2] + pack(">I", 4)
                + data[scalar.value_at:scalar.end] + b"\x00" + data[scalar.end:])
    duplicate = (data[:scalar.count_at] + b"\x00\x02"
                 + data[scalar.attribute_at:scalar.end] * 2 + data[scalar.end:])
    nested = defaults["nested"]
    if data[nested.value_at] != ord("@") or data[nested.value_at + 3:nested.value_at + 5] != b"\x00\x01":
        raise ValueError("expected an annotation default with one explicit element")
    element_name = nested.value_at + 5
    invalid_name = data[:element_name] + b"\x00\x00" + data[element_name + 2:]
    wide = defaults["wide"]
    if wide.end - wide.value_at != 3 or data[wide.value_at] != ord("J"):
        raise ValueError("expected a long default attribute")
    wrong_type = data[:scalar.value_at] + data[wide.value_at:wide.end] + data[scalar.end:]
    return {
        "trailing": ("number", "TrailingBytes", trailing),
        "duplicate": ("number", "DuplicateAttribute", duplicate),
        "nested-name": ("nested", "InvalidConstantIndex", invalid_name),
        "wrong-type": ("number", "does not match its annotation element", wrong_type),
    }
