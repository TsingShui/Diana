"""Produce single-disk archives with independently checkable ZIP64 fields."""

import io
from pathlib import Path
import runpy
import struct
import sys
from zipfile import ZIP_DEFLATED, ZipFile


Class = runpy.run_path(
    str(Path(__file__).parents[1] / "physical-inventory" / "prepare.py")
)["Class"]


def simple_type(name, result):
    source = Class(name, "java/lang/Object")
    source.method(0x0009, "probe", "()I", b"\x10" + bytes((result,)) + b"\xac", stack=1)
    return source.bytes()


LARGE_NAME = "fixture/archive/Large.class"
LARGE = simple_type("fixture/archive/Large", 41)
NEIGHBOR_NAME = "fixture/archive/Neighbor.class"
NEIGHBOR = simple_type("fixture/archive/Neighbor", 22)


def large_count(out):
    with ZipFile(out / "large.jar", "w") as archive:
        for idx in range(70_000):
            archive.writestr(f"f/{idx}", b"")
        archive.writestr(LARGE_NAME, LARGE, compress_type=ZIP_DEFLATED)
        archive.writestr(NEIGHBOR_NAME, NEIGHBOR)
        archive.comment = b"comment with a signature-like trailer PK\x05\x06" + bytes(26)
    # Verify actual writer output with a second reader. Python's tail finder
    # cannot handle the deliberately ambiguous signature in the final comment.
    data = (out / "large.jar").read_bytes()
    actual_eocd = data.find(b"PK\x05\x06", data.rindex(b"PK\x06\x07"))
    assert actual_eocd > 0
    with ZipFile(io.BytesIO(data[:actual_eocd + 20] + b"\x00\x00")) as archive:
        assert len(archive.infolist()) == 70_002
        assert archive.read(LARGE_NAME) == LARGE


def write_archive(output):
    with ZipFile(output, "w", compression=ZIP_DEFLATED) as archive:
        with archive.open(LARGE_NAME, "w", force_zip64=True) as entry:
            entry.write(LARGE)
        archive.writestr(NEIGHBOR_NAME, NEIGHBOR)


def wide_fields(out):
    local = io.BytesIO()
    write_archive(local)
    data = local.getvalue()
    (original_count, original_size, directory) = struct.unpack_from("<HII", data, len(data) - 12)
    assert original_count == 2
    first = bytearray(data[directory:directory + 46])
    assert first[:4] == b"PK\x01\x02"
    compressed, plain, name, extra, comment = struct.unpack_from("<IIHHH", first, 20)
    assert plain == len(LARGE) and extra == 0 and comment == 0
    next_header = directory + 46 + name
    assert data[next_header:next_header + 4] == b"PK\x01\x02"

    # Put an unrelated extra field before the field with the replacements.
    prefix = struct.pack("<HHB", 0xCAFE, 1, 7)
    wide = struct.pack("<HHQQQI", 1, 28, plain, compressed, 0, 0)
    struct.pack_into("<II", first, 20, 0xFFFFFFFF, 0xFFFFFFFF)
    struct.pack_into("<H", first, 30, len(prefix + wide))
    struct.pack_into("<H", first, 34, 0xFFFF)
    struct.pack_into("<I", first, 42, 0xFFFFFFFF)
    directory_bytes = first + data[directory + 46:next_header] + prefix + wide
    directory_bytes += data[next_header:directory + original_size]
    size = len(directory_bytes)
    record = directory + size
    extension = struct.pack("<HI", 0xCAFE, 3) + b"old"
    end64 = struct.pack(
        "<4sQHHIIQQQQ",
        b"PK\x06\x06", 44 + len(extension), 45, 45, 0, 0, 2, 2, size, directory,
    ) + extension
    locator = struct.pack("<4sIQI", b"PK\x06\x07", 0, record, 1)
    end32 = struct.pack("<4sHHHHIIH", b"PK\x05\x06", 0, 0, 2, 2, size, directory, 0)
    edited = bytearray(data[:directory] + directory_bytes + end64 + locator + end32)
    (out / "wide.jar").write_bytes(edited)
    with ZipFile(io.BytesIO(edited)) as archive:
        assert [entry.filename for entry in archive.infolist()] == [LARGE_NAME, NEIGHBOR_NAME]
        assert archive.read(LARGE_NAME) == LARGE

    local_name_len = struct.unpack_from("<H", edited, 26)[0]
    local_size = 30 + local_name_len + 4
    bad_local = edited.copy()
    struct.pack_into("<Q", bad_local, local_size, plain + 1)
    (out / "bad-local.jar").write_bytes(bad_local)
    bad_extra = edited.copy()
    struct.pack_into("<H", bad_extra, local_size - 2, 0x7FFF)
    (out / "bad-extra.jar").write_bytes(bad_extra)
    bad_legacy = edited.copy()
    struct.pack_into("<I", bad_legacy, 22, plain + 1)
    (out / "bad-legacy.jar").write_bytes(bad_legacy)

    huge = edited.copy()
    struct.pack_into("<QQ", huge, record + 24, 1 << 63, 1 << 63)
    struct.pack_into("<HH", huge, len(huge) - 14, 0xFFFF, 0xFFFF)
    (out / "huge-count.jar").write_bytes(huge)

    broken = edited.copy()
    struct.pack_into("<Q", broken, len(broken) - 34, (1 << 64) - 1)
    (out / "bad-locator.jar").write_bytes(broken)


class StreamingBytes(io.BytesIO):
    def seekable(self):
        return False

    def seek(self, *_args):
        raise io.UnsupportedOperation("streaming fixture")


def streaming(out):
    stream = StreamingBytes()
    write_archive(stream)
    data = stream.getvalue()
    local_name_len = struct.unpack_from("<H", data, 26)[0]
    assert struct.unpack_from("<H", data, 6)[0] & 8
    assert struct.unpack_from("<QQ", data, 34 + local_name_len) == (0, 0)
    (out / "stream.jar").write_bytes(data)
    with ZipFile(io.BytesIO(data)) as archive:
        assert archive.read(LARGE_NAME) == LARGE


if __name__ == "__main__":
    action, root = sys.argv[1:]
    out = Path(root)
    out.mkdir(parents=True, exist_ok=True)
    {"count": large_count, "fields": wide_fields, "stream": streaming}[action](out)
