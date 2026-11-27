"""Compile declaration annotations and damage one independently decoded value."""

from pathlib import Path
from struct import pack
import subprocess
import sys
from zipfile import ZIP_DEFLATED, ZipFile


def compile_fixture(classes, names):
    source = Path(__file__).with_name("Annotated.java")
    subprocess.run([
        "java", "-m", "jdk.compiler/com.sun.tools.javac.Main",
        "-source", "17", "-target", "17", "-Xlint:-options",
        *names, "-d", str(classes), str(source),
    ], check=True)


def prepare(destination):
    destination.mkdir(parents=True, exist_ok=True)
    classes = destination / "classes"
    compile_fixture(classes, ["-parameters"])
    classes = classes / "fixture/annotations"
    hidden = (classes / "Hidden.class").read_bytes()
    name = b"MethodParameters"
    unrecognized = b"IgnoredParameter"
    original = b"\x01" + pack(">H", len(name)) + name
    replacement = b"\x01" + pack(">H", len(unrecognized)) + unrecognized
    assert len(replacement) == len(original) and hidden.count(original) == 1
    hidden_without_names = hidden.replace(original, replacement, 1)

    for name in ("annotations", "damaged", "unaligned"):
        with ZipFile(destination / f"{name}.jar", "x", ZIP_DEFLATED) as archive:
            for type_name in ("Annotated", "Rich", "Nested", "Quiet", "TypeUsage", "Hidden"):
                contents = (classes / f"{type_name}.class").read_bytes()
                if name == "damaged" and type_name == "Annotated":
                    marker = b"class-visible-marker"
                    original = b"\x01" + pack(">H", len(marker)) + marker
                    malformed = original[:-1] + b"\x00"
                    assert contents.count(original) == 1
                    contents = contents.replace(original, malformed, 1)
                if name == "unaligned" and type_name == "Hidden":
                    contents = hidden_without_names
                archive.writestr(f"fixture/annotations/{type_name}.class", contents)


if __name__ == "__main__":
    prepare(Path(sys.argv[1]))
