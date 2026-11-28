"""Independent JVM method inventory using the JDK class-file reader."""

from pathlib import Path
from tempfile import TemporaryDirectory
from zipfile import BadZipFile, ZIP_DEFLATED, ZIP_STORED, ZipFile

from cfr import JAVAP, parse_inventory, run_java

CLASS_BYTES = 16 * 1024 * 1024
ARCHIVE_CLASS_BYTES = 256 * 1024 * 1024


def class_methods(path: Path, timeout: int) -> list[tuple[str, str]]:
    _, rows = parse_inventory(run_java([*JAVAP, str(path)], timeout))
    return [(row["at"], row["member"]) for row in rows]


def methods(path: Path, timeout: int) -> list[tuple[str, str]]:
    if path.suffix.lower() == ".class":
        if path.stat().st_size > CLASS_BYTES:
            raise ValueError("class file exceeds the JVM oracle size bound")
        return class_methods(path, timeout)
    with TemporaryDirectory(prefix="diana-jvm-inventory-") as temporary:
        scratch = Path(temporary) / "Input.class"
        try:
            with ZipFile(path) as jar:
                return archive_methods(jar, scratch, timeout)
        except (BadZipFile, NotImplementedError, EOFError) as error:
            raise ValueError(f"JAR inventory is unavailable: {error}") from error


def archive_methods(jar: ZipFile, scratch: Path, timeout: int) -> list[tuple[str, str]]:
    entries = jar.infolist()
    if any(entry.orig_filename.lower().endswith(".jar") for entry in entries):
        raise ValueError("the independent JAR reader covers flat archives")
    result = []
    total = 0
    for index, entry in enumerate(entries):
        if entry.is_dir() or not entry.orig_filename.endswith(".class"):
            continue
        total += entry.file_size
        if entry.file_size > CLASS_BYTES or total > ARCHIVE_CLASS_BYTES:
            raise ValueError("class data exceeds the JVM oracle size bounds")
        if entry.flag_bits & 1 or entry.compress_type not in (ZIP_STORED, ZIP_DEFLATED):
            raise ValueError(f"encrypted or unsupported class entry at j{index}")
        with jar.open(entry) as stream:
            data = stream.read(entry.file_size + 1)
        if len(data) != entry.file_size:
            raise ValueError(f"class entry size changed at j{index}")
        scratch.write_bytes(data)
        rows = class_methods(scratch, timeout)
        result.extend((f"j{index}{at[2:]}", member) for at, member in rows)
    return result
