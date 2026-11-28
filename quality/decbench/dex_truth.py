"""Read defined DEX method identities for benchmark expectations.

The reader follows the standalone layout in https://source.android.com/docs/core/runtime/dex-format.
It reads declarations and their type descriptors without decoding method bodies.
"""

from __future__ import annotations

from dataclasses import dataclass
from mmap import ACCESS_READ, mmap
from os import fstat
from pathlib import Path
import re
from stat import S_ISREG
from struct import Struct
from zipfile import BadZipFile, ZIP_DEFLATED, ZIP_STORED, ZipFile
from zlib import error as DeflateError


HEADER_SIZE = 0x70
VERSIONS = {b"035", b"037", b"038", b"039", b"040"}
U16 = Struct("<H")
U32 = Struct("<I")
TABLE = Struct("<II")
METHOD = Struct("<HHI")
PROTO = Struct("<III")
APK_ROOT = re.compile(r"classes(?:[2-9]|[1-9][0-9]+)?\.dex")
# Bounds for the benchmark reader. The CLI's APK support has its own limits.
APK_FILE_BYTES = 256 * 1024 * 1024
APK_ROOT_BYTES = 64 * 1024 * 1024
APK_TOTAL_ROOT_BYTES = 256 * 1024 * 1024


class DexError(ValueError):
    """The oracle could not establish an inventory."""


class DexMalformed(DexError):
    """Invalid data in the declarations needed by the oracle."""


class DexUnsupported(DexError):
    """The input uses a layout outside the standalone DEX oracle."""


class ApkMalformed(DexMalformed):
    """The oracle could not read the APK directory or a ZIP member."""


class ApkUnsupported(DexUnsupported):
    """The APK is outside the benchmark oracle's supported ZIP layouts or bounds."""


@dataclass(frozen=True)
class Table:
    count: int
    start: int
    stride: int
    name: str

    def offset(self, index: int) -> int:
        if not 0 <= index < self.count:
            raise DexMalformed(f"{self.name} index {index} exceeds {self.count} entries")
        return self.start + index * self.stride


class Dex:
    def __init__(self, data: mmap | bytes):
        self.data = data
        magic = data[:8]
        if magic.startswith(b"cdex"):
            raise DexUnsupported("compact DEX layout")
        if len(magic) != 8 or magic[:4] != b"dex\n" or magic[7] != 0:
            raise DexMalformed("missing DEX magic")
        if magic[4:7] not in VERSIONS:
            raise DexUnsupported(f"DEX version {magic[4:7]!r}")
        self.range(0, HEADER_SIZE, len(data), "header")
        file_size, header_size, endian = PROTO.unpack_from(data, 0x20)
        if endian == 0x78563412:
            raise DexUnsupported("big-endian DEX")
        if endian != 0x12345678 or header_size != HEADER_SIZE or file_size < HEADER_SIZE:
            raise DexMalformed("invalid header size or endian tag")
        if file_size < len(data):
            raise DexUnsupported("bytes after the declared standalone DEX image")
        if file_size > len(data):
            raise DexMalformed("truncated DEX image")

        data_size, self.data_start = TABLE.unpack_from(data, 0x68)
        self.data_end = self.data_start + data_size
        self.range(self.data_start, data_size, file_size, "data section")
        if (data_size and (self.data_start < HEADER_SIZE or self.data_start % 4)) or (
            not data_size and self.data_start
        ):
            raise DexMalformed("invalid data section offset")

        self.strings = self.table(0x38, 4, "string_ids")
        self.types = self.table(0x40, 4, "type_ids")
        self.protos = self.table(0x48, 12, "proto_ids")
        self.fields = self.table(0x50, 8, "field_ids")
        self.method_ids = self.table(0x58, 8, "method_ids")
        self.classes = self.table(0x60, 32, "class_defs")
        expected_start = HEADER_SIZE
        for table in (self.strings, self.types, self.protos, self.fields, self.method_ids, self.classes):
            if table.count:
                if table.start < expected_start:
                    raise DexMalformed(f"overlapping or out-of-order {table.name}")
                expected_start = table.start + table.count * table.stride
        self.decoded_strings: dict[int, str] = {}
        self.decoded_protos: dict[int, str] = {}

    @staticmethod
    def range(start: int, size: int, limit: int, what: str) -> None:
        if start < 0 or size < 0 or start + size > limit:
            raise DexMalformed(f"{what} exceeds its section at byte {start}")

    def table(self, offset: int, stride: int, name: str) -> Table:
        count, start = TABLE.unpack_from(self.data, offset)
        if count:
            if start < HEADER_SIZE or start % 4:
                raise DexMalformed(f"invalid {name} offset {start}")
            self.range(start, count * stride, self.data_start or len(self.data), name)
        elif start:
            raise DexMalformed(f"empty {name} has a nonzero offset")
        return Table(count, start, stride, name)

    def data_range(self, start: int, size: int, what: str) -> None:
        if start < self.data_start:
            raise DexMalformed(f"{what} precedes the data section at byte {start}")
        self.range(start, size, self.data_end, what)

    def uleb(self, start: int) -> tuple[int, int]:
        value = 0
        for byte in range(5):
            self.data_range(start + byte, 1, "ULEB128")
            octet = self.data[start + byte]
            if byte == 4 and octet > 0x0F:
                break
            value |= (octet & 0x7F) << (7 * byte)
            if octet < 0x80:
                return value, start + byte + 1
        raise DexMalformed(f"ULEB128 exceeds 32 bits at byte {start}")

    def string(self, index: int) -> str:
        if index in self.decoded_strings:
            return self.decoded_strings[index]
        start = U32.unpack_from(self.data, self.strings.offset(index))[0]
        count, start = self.uleb(start)
        self.data_range(start, count + 1, "string_data")
        units = bytearray()
        for _ in range(count):
            self.data_range(start, 1, "string_data")
            lead = self.data[start]
            if 0 < lead < 0x80:
                unit = lead
                start += 1
            elif 0xC0 <= lead < 0xF0:
                width = 2 if lead < 0xE0 else 3
                self.data_range(start, width, "string_data")
                unit = lead & (0x1F if width == 2 else 0x0F)
                for tail in self.data[start + 1:start + width]:
                    if tail & 0xC0 != 0x80:
                        raise DexMalformed(f"invalid MUTF-8 continuation at byte {start}")
                    unit = unit << 6 | (tail & 0x3F)
                if (width == 2 and 0 < unit < 0x80) or (width == 3 and unit < 0x800):
                    raise DexMalformed(f"overlong MUTF-8 at byte {start}")
                start += width
            else:
                raise DexMalformed(f"invalid MUTF-8 leading byte at byte {start}")
            units += U16.pack(unit)
        self.data_range(start, 1, "string terminator")
        if self.data[start] != 0:
            raise DexMalformed(f"missing string terminator at byte {start}")
        text = display(units.decode("utf-16-le", "surrogatepass"))
        self.decoded_strings[index] = text
        return text

    def descriptor(self, index: int) -> str:
        return self.string(U32.unpack_from(self.data, self.types.offset(index))[0])

    def prototype(self, index: int) -> str:
        if index in self.decoded_protos:
            return self.decoded_protos[index]
        _, returns, parameters = PROTO.unpack_from(self.data, self.protos.offset(index))
        arguments = ""
        if parameters:
            if parameters % 4:
                raise DexMalformed(f"unaligned parameter type_list at byte {parameters}")
            self.data_range(parameters, 4, "parameter count")
            count = U32.unpack_from(self.data, parameters)[0]
            self.data_range(parameters + 4, count * 2, "parameter type_list")
            arguments = "".join(
                self.descriptor(U16.unpack_from(self.data, entry)[0])
                for entry in range(parameters + 4, parameters + 4 + count * 2, 2)
            )
        result = f"({arguments}){self.descriptor(returns)}"
        self.decoded_protos[index] = result
        return result

    def class_methods(self, owner: int, start: int) -> tuple[list[tuple[int, str]], int]:
        counts = []
        for _ in range(4):
            count, start = self.uleb(start)
            counts.append(count)
        field_count = sum(counts[:2])
        method_count = sum(counts[2:])
        if field_count > self.fields.count or method_count > self.method_ids.count:
            raise DexMalformed(f"class_data counts exceed the ID tables at byte {start}")
        self.data_range(start, 2 * field_count + 3 * method_count, "encoded members")

        for _ in range(field_count * 2):
            _, start = self.uleb(start)
        result = []
        for count in counts[2:]:
            index = 0
            for position in range(count):
                delta, start = self.uleb(start)
                if position and delta == 0:
                    raise DexMalformed(f"repeated method index {index} in class_data")
                index += delta
                _, start = self.uleb(start)  # access_flags
                _, start = self.uleb(start)  # code_off is outside this oracle
                class_id, proto_id, name_id = METHOD.unpack_from(
                    self.data, self.method_ids.offset(index),
                )
                if class_id != owner:
                    raise DexMalformed(f"method {index} has the wrong declaring class")
                signature = f"{self.descriptor(owner)}->{self.string(name_id)}{self.prototype(proto_id)}"
                result.append((index, signature))
        return result, field_count

    def methods(self, entry: int = 0) -> list[tuple[str, str]]:
        owners = set()
        class_data = set()
        methods_seen = set()
        field_count = 0
        result = []
        for ordinal in range(self.classes.count):
            offset = self.classes.offset(ordinal)
            owner = U32.unpack_from(self.data, offset)[0]
            self.types.offset(owner)
            if owner in owners:
                raise DexMalformed(f"repeated class definition for type ID {owner}")
            owners.add(owner)
            start = U32.unpack_from(self.data, offset + 24)[0]
            if not start:
                continue
            if start in class_data:
                raise DexMalformed(f"repeated class_data at byte {start}")
            class_data.add(start)
            methods, count = self.class_methods(owner, start)
            field_count += count
            if field_count > self.fields.count:
                raise DexMalformed("encoded field count exceeds the field ID table")
            for index, signature in methods:
                if index in methods_seen:
                    raise DexMalformed(f"repeated definition of method {index}")
                methods_seen.add(index)
                result.append((f"e{entry}:l0:c{ordinal}:m{index}", signature))
        return result


def display(text: str) -> str:
    escapes = {"\\": "\\\\", "\t": "\\t", "\r": "\\r", "\n": "\\n", "\0": "\\0"}
    return "".join(
        f"\\u{ord(ch):04x}" if 0xD800 <= ord(ch) <= 0xDFFF else escapes.get(ch, ch)
        for ch in text
    )


def methods(path: Path) -> list[tuple[str, str]]:
    """Read the locators and descriptors of defined methods in one standalone DEX."""
    if not path.is_file():
        raise DexUnsupported("DEX input must be a regular file")
    with path.open("rb") as stream:
        info = fstat(stream.fileno())
        if not S_ISREG(info.st_mode):
            raise DexUnsupported("DEX input must be a regular file")
        if not info.st_size:
            raise DexMalformed("empty DEX input")
        with mmap(stream.fileno(), 0, access=ACCESS_READ) as data:
            return Dex(data).methods()


def apk_methods(path: Path) -> list[tuple[str, str]]:
    """Read each supported APK root image using its central-directory position."""
    if not path.is_file():
        raise ApkUnsupported("APK input must be a regular file")
    with path.open("rb") as stream:
        info = fstat(stream.fileno())
        if not S_ISREG(info.st_mode):
            raise ApkUnsupported("APK input must be a regular file")
        if info.st_size > APK_FILE_BYTES:
            raise ApkUnsupported("APK exceeds the benchmark archive size bound")
        try:
            with ZipFile(stream) as archive:
                return archive_methods(archive)
        except (BadZipFile, EOFError, UnicodeDecodeError) as error:
            raise ApkMalformed(f"invalid APK ZIP directory: {error}") from error
        except NotImplementedError as error:
            raise ApkUnsupported(f"unsupported APK ZIP directory: {error}") from error


def archive_methods(archive: ZipFile) -> list[tuple[str, str]]:
    result = []
    roots = 0
    total_size = 0
    for physical, entry in enumerate(archive.infolist()):
        name = entry.orig_filename
        if APK_ROOT.fullmatch(name) is None:
            continue
        roots += 1
        location = f"e{physical} {name}"
        if entry.flag_bits & 0x1 or entry.compress_type not in (ZIP_STORED, ZIP_DEFLATED):
            raise ApkUnsupported(f"{location}: encrypted or unsupported ZIP member")
        total_size += entry.file_size
        if entry.file_size > APK_ROOT_BYTES or total_size > APK_TOTAL_ROOT_BYTES:
            raise ApkUnsupported(f"{location}: root DEX data exceeds benchmark size bounds")
        try:
            with archive.open(entry) as stream:
                data = stream.read(entry.file_size + 1)
        except (BadZipFile, DeflateError, EOFError, UnicodeDecodeError) as error:
            raise ApkMalformed(f"{location}: invalid ZIP member: {error}") from error
        except NotImplementedError as error:
            raise ApkUnsupported(f"{location}: unsupported ZIP member: {error}") from error
        if len(data) != entry.file_size:
            raise ApkMalformed(f"{location}: ZIP member size differs from its directory record")
        try:
            result.extend(Dex(data).methods(physical))
        except DexError as error:
            raise type(error)(f"{location}: {error}") from error
    if not roots:
        raise ApkUnsupported("APK has no root classes.dex or classes<N>.dex records")
    return result
