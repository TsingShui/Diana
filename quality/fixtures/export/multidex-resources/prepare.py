"""Assemble an export challenge from public fixture DEX files and authored bytes."""

import sys
from zlib import crc32
from pathlib import Path
from struct import pack
from zipfile import ZIP_BZIP2, ZIP_DEFLATED, ZIP_STORED, ZipFile, ZipInfo


FIXTURES = Path(__file__).resolve().parents[2]


def manifest():
    # Android binary XML for <manifest></manifest>. One UTF-8 pool entry;
    # both element records refer to its index, without a namespace or attributes.
    pool = pack("<HHI6I", 1, 28, 44, 1, 0, 0x100, 32, 0, 0)
    pool += b"\x08\x08manifest\x00\x00"
    absent = 0xFFFFFFFF
    start = pack("<HHIIIII6H", 0x102, 16, 36, 1, absent, absent, 0, 20, 20, 0, 0, 0, 0)
    end = pack("<HHIIIII", 0x103, 16, 24, 1, absent, absent, 0)
    chunks = pool + start + end
    return pack("<HHI", 3, 8, 8 + len(chunks)) + chunks


def write_entry(archive, name, data, compression):
    entry = ZipInfo(name)
    entry.create_system = 3
    archive.writestr(entry, data, compression)


def prepare(archive, scenario):
    arithmetic = (FIXTURES / "atoms/expr-arith/classes.dex").read_bytes()
    branches = (FIXTURES / "branches/branch-ifz/classes.dex").read_bytes()
    gap = (FIXTURES / "classes/direct-initialization/classes.dex").read_bytes()
    with ZipFile(archive, "x") as output:
        write_entry(output, "classes.dex", arithmetic, ZIP_DEFLATED)
        write_entry(output, "AndroidManifest.xml", manifest(), ZIP_DEFLATED)
        write_entry(output, "resources.arsc", b"opaque table\x00\xff", ZIP_STORED)
        write_entry(output, "assets/nested.dex", gap, ZIP_DEFLATED)
        # Cross several transfer-buffer boundaries and finish with a short tail.
        write_entry(output, "res/raw/payload", bytes(range(256)) * 2048 + b"end", ZIP_DEFLATED)
        write_entry(output, "lib/arm64-v8a/libsample.so", b"\x7fELF\x02\x01\x00\xff", ZIP_STORED)
        if scenario == "partial":
            write_entry(output, "assets/damaged", b"stored-crc-fixture", ZIP_STORED)
            write_entry(output, "res/raw/unsupported", b"bzip2 payload", ZIP_BZIP2)
            write_entry(output, "classes3.dex", gap, ZIP_DEFLATED)
            write_entry(output, "classes4.dex", b"dex\n035\x00", ZIP_STORED)
            write_entry(output, "classes5.dex", arithmetic, ZIP_STORED)
        elif scenario == "duplicate":
            # Parent and Child have the same descriptors across both roots.
            # Only the second Parent records event(2) before storing the value.
            effect = (FIXTURES / "exceptions/direct-initialization-effects/classes.dex").read_bytes()
            write_entry(output, "classes3.dex", gap, ZIP_STORED)
            write_entry(output, "classes4.dex", effect, ZIP_DEFLATED)
        elif scenario == "nested":
            member = (FIXTURES / "classes/class-inner-static/classes.dex").read_bytes()
            write_entry(output, "classes3.dex", member, ZIP_DEFLATED)
        elif scenario == "unsafe-path":
            write_entry(output, "../../escape", b"outside", ZIP_STORED)
        elif scenario != "complete":
            raise ValueError(scenario)
        write_entry(output, "classes2.dex", branches, ZIP_STORED)
    if scenario == "partial":
        data = bytearray(archive.read_bytes())
        data[data.index(b"stored-crc-fixture")] ^= 1
        # Keep the DEX intact, alter its declared CRC in both ZIP headers.
        name = b"classes5.dex"
        local = data.index(name) - 30
        central = data.index(name, local + 30 + len(name)) - 46
        for offset in (local + 14, central + 16):
            assert data[offset:offset + 4] == pack("<I", crc32(arithmetic))
            data[offset] ^= 1
        archive.write_bytes(data)


if __name__ == "__main__":
    prepare(Path(sys.argv[1]), sys.argv[2])
