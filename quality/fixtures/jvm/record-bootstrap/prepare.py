"""Compile record bootstraps and change independently identified original facts."""

from pathlib import Path
import shutil
from struct import unpack_from
import subprocess
import sys
from zipfile import ZIP_DEFLATED, ZipFile


def java_tool(tool):
    if executable := shutil.which(tool):
        return [executable]
    module = {
        "javac": "jdk.compiler/com.sun.tools.javac.Main",
        "javap": "jdk.jdeps/com.sun.tools.javap.Main",
    }[tool]
    return ["java", "-m", module]


def constants(data):
    count = unpack_from(">H", data, 8)[0]
    offset = 10
    slot = 1
    while slot < count:
        tag = data[offset]
        yield slot, tag, offset + 1
        offset += 1
        if tag == 1:
            offset += 2 + unpack_from(">H", data, offset)[0]
        else:
            offset += {
                3: 4, 4: 4, 5: 8, 6: 8, 7: 2, 8: 2, 9: 4, 10: 4,
                11: 4, 12: 4, 15: 3, 16: 2, 17: 4, 18: 4, 19: 2, 20: 2,
            }[tag]
        slot += 2 if tag in (5, 6) else 1


def prepare(directory):
    directory.mkdir(parents=True, exist_ok=True)
    source = Path(__file__).resolve().parent
    output = directory / "classes"
    subprocess.run([*java_tool("javac"), "-source", "17", "-target", "17", "-Xlint:-options", "-g", "-parameters",
                    "-d", str(output), *map(str, source.glob("*.java"))], check=True)
    probe = output / "fixture" / "record" / "RecordProbe.class"
    original = probe.read_bytes()
    empty = output / "fixture" / "record" / "Empty.class"
    notes = b"count;reading;label;values"
    assert original.count(notes) == 1
    contradictory = original.replace(notes, b"other;reading;label;values")
    (directory / "contradictory.class").write_bytes(contradictory)

    malformed = bytearray(original)
    index = next(offset for _, tag, offset in constants(original)
                 if tag == 15 and original[offset] == 1)
    # A REF_getField with zero reference_index has no original target.
    malformed[index + 1:index + 3] = b"\0\0"
    (directory / "malformed.class").write_bytes(malformed)
    with ZipFile(directory / "records.jar", "w", compression=ZIP_DEFLATED) as archive:
        for path in [probe, empty]:
            archive.write(path, path.relative_to(output).as_posix())


if __name__ == "__main__":
    prepare(Path(sys.argv[1]))
