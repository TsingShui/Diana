"""Compile the fixture with javac and give its class several physical locations."""

from io import BytesIO
from pathlib import Path
from struct import pack
import subprocess
import sys
from zipfile import ZIP_DEFLATED, ZIP_STORED, ZipFile


def prepare(destination):
    destination.mkdir(parents=True, exist_ok=True)
    source = Path(__file__).with_name("SourceRecovery.java")
    classes = destination / "classes"
    subprocess.run(
        [
            "java", "-m", "jdk.compiler/com.sun.tools.javac.Main",
            "-source", "17", "-target", "17", "-Xlint:-options",
            "-g", "-encoding", "UTF-8", "-d", str(classes),
            str(source), str(source.with_name("CustomBootstrap.java")),
            str(source.with_name("ExceptionChains.java")),
        ],
        check=True,
    )
    data = (classes / "fixture/jvm/SourceRecovery.class").read_bytes()
    with ZipFile(destination / "source.jar", "x") as jar:
        jar.writestr("assets/readme.txt", "Non-class resource\n", ZIP_STORED)
        jar.writestr("misleading/Serialized.class", data, ZIP_DEFLATED)

    with ZipFile(destination / "multi-release.jar", "x") as jar:
        jar.writestr("META-INF/MANIFEST.MF", b"Manifest-Version: 1.0\r\nMulti-Release: true\r\n\r\n")
        jar.writestr("fixture/jvm/SourceRecovery.class", data, ZIP_DEFLATED)
        jar.writestr("META-INF/versions/17/fixture/jvm/SourceRecovery.class", data, ZIP_DEFLATED)

    catalog = f"{'0' * 64}\tfixture:source\tfixture/source.jar\n"
    name = "fixture/jvm/SourceRecovery.class"
    nested = nested_jar([(name, data)])
    write_bundle(destination / "bundle.jar", catalog, nested)
    write_bundle(destination / "damaged-bundle.jar", catalog + "incomplete row\n", nested)

    bootstrap_name = "fixture/jvm/CustomBootstrap.class"
    bootstrap = (classes / bootstrap_name).read_bytes()
    opaque = replace_bootstrap_owner(data)
    opaque_entries = [(name, opaque), (bootstrap_name, bootstrap)]
    for entry, contents in opaque_entries:
        path = destination / "opaque-classes" / entry
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(contents)
    write_bundle(destination / "opaque-bundle.jar", catalog, nested_jar(opaque_entries))


def replace_bootstrap_owner(data):
    # Replace one CONSTANT_Utf8 record. Pool indexes and every bytecode
    # instruction stay the same while the bootstrap handle gains a new owner.
    original = b"java/lang/invoke/StringConcatFactory"
    replacement = b"fixture/jvm/CustomBootstrap"
    old = b"\x01" + pack(">H", len(original)) + original
    new = b"\x01" + pack(">H", len(replacement)) + replacement
    assert data.count(old) == 1
    return data.replace(old, new, 1)


def nested_jar(entries):
    output = BytesIO()
    with ZipFile(output, "w") as jar:
        for name, data in entries:
            jar.writestr(name, data, ZIP_DEFLATED)
    return output.getvalue()


def write_bundle(destination, catalog, nested):
    with ZipFile(destination, "x") as bundle:
        bundle.writestr("META-INF/libraries.list", catalog)
        bundle.writestr("META-INF/libraries/fixture/source.jar", nested, ZIP_DEFLATED)


if __name__ == "__main__":
    prepare(Path(sys.argv[1]))
