"""Read ordinary invoke sites in supported straight-line DEX methods.

Instruction widths and code item offsets follow the published Dalvik format.
The inventory reader supplies type, prototype, string and table validation.
"""

from mmap import ACCESS_READ, mmap
from pathlib import Path
import re
from zipfile import ZIP_DEFLATED, ZIP_STORED, ZipFile

from dex_truth import APK_FILE_BYTES, APK_ROOT, APK_ROOT_BYTES, Dex, DexMalformed, DexUnsupported
from dex_truth import METHOD, U16, U32

LOCATOR = re.compile(r"e(0|[1-9][0-9]*):l0:c(0|[1-9][0-9]*):m(0|[1-9][0-9]*)")
MAX_CODE_UNITS = 65_535
# The decoder skips operands for instructions irrelevant to invoke evidence.
# Branches, payloads and dynamic invokes are excluded from the linear witness.
WIDTHS = {
    0x00: 1, 0x01: 1, 0x02: 2, 0x03: 3, 0x04: 1, 0x05: 2, 0x06: 3,
    0x07: 1, 0x08: 2, 0x09: 3, 0x13: 2, 0x14: 3, 0x15: 2, 0x16: 2,
    0x17: 3, 0x18: 5, 0x19: 2, 0x1a: 2, 0x1b: 3, 0x1c: 2, 0x1f: 2,
    0x20: 2, 0x21: 1, 0x22: 2, 0x23: 2, 0x24: 3, 0x25: 3,
    0xfe: 2, 0xff: 2,
    **dict.fromkeys(range(0x0a, 0x13), 1),
    **dict.fromkeys(range(0x2d, 0x32), 2),
    **dict.fromkeys(range(0x44, 0x6e), 2),
    **dict.fromkeys(range(0x6e, 0x73), 3),
    **dict.fromkeys(range(0x74, 0x79), 3),
    **dict.fromkeys(range(0x7b, 0x90), 1),
    **dict.fromkeys(range(0x90, 0xb0), 2),
    **dict.fromkeys(range(0xb0, 0xd0), 1),
    **dict.fromkeys(range(0xd0, 0xe3), 2),
}
INVOKES = frozenset(range(0x6e, 0x73)) | frozenset(range(0x74, 0x79))


def method_ref(dex: Dex, index: int) -> tuple[str, str, str]:
    owner, proto, name = METHOD.unpack_from(dex.data, dex.method_ids.offset(index))
    return dex.descriptor(owner), dex.string(name), dex.prototype(proto)


def method_code(dex: Dex, class_index: int, method_index: int, member: str) -> int:
    definition = dex.classes.offset(class_index)
    owner = U32.unpack_from(dex.data, definition)[0]
    actual_owner, name, descriptor = method_ref(dex, method_index)
    if dex.descriptor(owner) != actual_owner or member != f"{actual_owner}->{name}{descriptor}":
        raise DexMalformed("selected DEX declaration differs from the authored member")
    start = U32.unpack_from(dex.data, definition + 24)[0]
    if not start:
        raise DexMalformed("selected DEX class has no encoded definitions")
    counts = []
    for _ in range(4):
        count, start = dex.uleb(start)
        counts.append(count)
    fields, methods = sum(counts[:2]), sum(counts[2:])
    if fields > dex.fields.count or methods > dex.method_ids.count:
        raise DexMalformed("encoded members exceed the declaration tables")
    dex.data_range(start, 2 * fields + 3 * methods, "encoded declarations")
    for _ in range(2 * fields):
        _, start = dex.uleb(start)
    result = None
    for count in counts[2:]:
        index = 0
        for position in range(count):
            delta, start = dex.uleb(start)
            if position and not delta:
                raise DexMalformed("repeated encoded method")
            index += delta
            dex.method_ids.offset(index)
            _, start = dex.uleb(start)  # access flags
            code, start = dex.uleb(start)
            if index == method_index:
                if result is not None:
                    raise DexMalformed("selected DEX method has multiple definitions")
                result = code
    if result is None:
        raise DexMalformed("selected DEX method definition is absent")
    if not result:
        raise DexUnsupported("selected DEX method has no implementation")
    return result


def dispatch(opcode: int, name: str) -> str:
    if name == "<init>":
        return "constructor"
    if opcode in (0x6f, 0x75):
        return "super"
    if opcode in (0x71, 0x77):
        return "static"
    return "instance"


def invokes(dex: Dex, code: int) -> list[dict]:
    if code % 4:
        raise DexMalformed("unaligned DEX code item")
    dex.data_range(code, 16, "code item")
    if U16.unpack_from(dex.data, code + 6)[0]:
        raise DexUnsupported("call alignment requires an exception-free straight-line DEX method")
    count = U32.unpack_from(dex.data, code + 12)[0]
    if not count:
        raise DexMalformed("DEX code item contains no instructions")
    if count > MAX_CODE_UNITS:
        raise DexUnsupported("DEX code exceeds the call oracle bound")
    start = code + 16
    dex.data_range(start, count * 2, "instructions")
    result = []
    pc = 0
    while pc < count:
        word = U16.unpack_from(dex.data, start + pc * 2)[0]
        opcode = word & 0xff
        width = WIDTHS.get(opcode)
        if width is None or (opcode == 0 and word):
            raise DexUnsupported(f"call alignment cannot model DEX opcode 0x{opcode:02x} at code unit {pc}")
        if pc + width > count:
            raise DexMalformed(f"truncated DEX instruction at code unit {pc}")
        if 0x0e <= opcode <= 0x11 and pc != count - 1:
            raise DexUnsupported("call alignment cannot model DEX instructions after a return")
        if opcode in INVOKES:
            index = U16.unpack_from(dex.data, start + (pc + 1) * 2)[0]
            owner, name, descriptor = method_ref(dex, index)
            result.append({
                "pc": pc, "owner": owner, "name": name, "descriptor": descriptor,
                "dispatch": dispatch(opcode, name),
            })
        pc += width
    if not 0x0e <= opcode <= 0x11:
        raise DexMalformed("straight-line DEX implementation does not end in a return")
    return result


def read_calls(path: Path, location: str, member: str) -> list[dict]:
    match = LOCATOR.fullmatch(location)
    if match is None:
        raise DexUnsupported("call oracle requires a DEX method locator for layout l0")
    entry, owner, method = map(int, match.groups())
    if not path.is_file():
        raise DexUnsupported("call oracle requires a regular input file")
    if path.suffix.lower() == ".dex":
        if entry:
            raise DexUnsupported("standalone DEX method locators use entry e0")
        with path.open("rb") as stream:
            if stream.seek(0, 2) == 0:
                raise DexMalformed("empty DEX")
            with mmap(stream.fileno(), 0, access=ACCESS_READ) as data:
                dex = Dex(data)
                return invokes(dex, method_code(dex, owner, method, member))
    if path.suffix.lower() != ".apk" or path.stat().st_size > APK_FILE_BYTES:
        raise DexUnsupported("call oracle covers standalone DEX and bounded APK files")
    with ZipFile(path) as archive:
        entries = archive.infolist()
        if entry >= len(entries):
            raise DexMalformed("physical APK entry position is absent")
        selected = entries[entry]
        if (APK_ROOT.fullmatch(selected.orig_filename) is None
                or selected.file_size > APK_ROOT_BYTES or selected.flag_bits & 1
                or selected.compress_type not in (ZIP_STORED, ZIP_DEFLATED)):
            raise DexUnsupported("the physical APK entry is outside the supported DEX roots")
        with archive.open(selected) as stream:
            data = stream.read(APK_ROOT_BYTES + 1)
        if len(data) != selected.file_size:
            raise DexMalformed("physical DEX root size differs from the ZIP directory")
    dex = Dex(data)
    return invokes(dex, method_code(dex, owner, method, member))
