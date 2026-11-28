"""Compile the fixture with javac and give its class several physical locations."""

import argparse
from io import BytesIO
import os
from pathlib import Path
from struct import pack
import subprocess
from zipfile import ZIP_DEFLATED, ZIP_STORED, ZipFile, ZipInfo


JAVA_ENV_OVERRIDES = ("CLASSPATH", "JDK_JAVA_OPTIONS", "JAVA_TOOL_OPTIONS", "_JAVA_OPTIONS")
JAVAC = ("java", "-m", "jdk.compiler/com.sun.tools.javac.Main")
VARIANTS = {"debug": "-g", "stripped": "-g:none"}


def java_environment():
    return {key: value for key, value in os.environ.items() if key not in JAVA_ENV_OVERRIDES}


def compile_command(destination, variant, sources):
    return [
        *JAVAC, "-source", "17", "-target", "17", "-Xlint:-options",
        VARIANTS[variant], "-encoding", "UTF-8", "-d", str(destination),
        *map(str, sources),
    ]


def write_entry(archive, name, data, compression=ZIP_STORED):
    # Fix entry metadata so repeated runs can identify the same prepared input.
    entry = ZipInfo(name)
    entry.create_system = 3
    archive.writestr(entry, data, compression)


def prepare(destination, variant="debug"):
    destination.mkdir(parents=True, exist_ok=True)
    fixture = Path(__file__).resolve().parent
    source_files = [fixture / name for name in (
        "SourceRecovery.java", "CustomBootstrap.java", "ExceptionChains.java",
        "TypedLocals.java", "EffectSequences.java",
    )]
    classes = destination / "classes"
    subprocess.run(
        compile_command(classes, variant, source_files),
        env=java_environment(), check=True,
    )
    data = (classes / "fixture/jvm/SourceRecovery.class").read_bytes()
    with ZipFile(destination / "source.jar", "x") as jar:
        write_entry(jar, "assets/readme.txt", "Non-class resource\n")
        write_entry(jar, "misleading/Serialized.class", data, ZIP_DEFLATED)

    with ZipFile(destination / "multi-release.jar", "x") as jar:
        write_entry(jar, "META-INF/MANIFEST.MF", b"Manifest-Version: 1.0\r\nMulti-Release: true\r\n\r\n")
        write_entry(jar, "fixture/jvm/SourceRecovery.class", data, ZIP_DEFLATED)
        write_entry(jar, "META-INF/versions/17/fixture/jvm/SourceRecovery.class", data, ZIP_DEFLATED)

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
            write_entry(jar, name, data, ZIP_DEFLATED)
    return output.getvalue()


def write_bundle(destination, catalog, nested):
    with ZipFile(destination, "x") as bundle:
        write_entry(bundle, "META-INF/libraries.list", catalog)
        write_entry(bundle, "META-INF/libraries/fixture/source.jar", nested, ZIP_DEFLATED)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("destination", type=Path)
    parser.add_argument("--variant", choices=VARIANTS, default="debug")
    args = parser.parse_args()
    prepare(args.destination, args.variant)


if __name__ == "__main__":
    main()
