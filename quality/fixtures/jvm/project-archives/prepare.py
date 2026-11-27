"""Build independently authored JVM archives at physical project coordinates."""

import io
import json
from pathlib import Path
import runpy
import sys
from zipfile import ZIP_DEFLATED, ZIP_STORED, ZipFile


recipe = runpy.run_path(
    str(Path(__file__).parents[1] / "physical-inventory" / "prepare.py")
)
Class, u2 = recipe["Class"], recipe["u2"]


def number(name, value):
    source = Class(name, "java/lang/Object")
    source.method(0x0009, "probe", "()I", b"\x10" + bytes((value,)) + b"\xac", stack=1)
    return source.bytes()


def caller():
    source = Class("fixture/project/Caller", "java/lang/Object")
    target = source.reference(10, "fixture/project/Library", "probe", "()I")
    source.method(0x0009, "probe", "()I", b"\xb8" + u2(target) + b"\xac", stack=1)
    return source.bytes()


def zip_bytes(entries):
    output = io.BytesIO()
    with ZipFile(output, "w") as archive:
        for name, data, method in entries:
            archive.writestr(name, data, compress_type=method)
    return output.getvalue()


def nested(out):
    library = zip_bytes([
        ("fixture/project/Library.class", number("fixture/project/Library", 37), ZIP_DEFLATED),
    ])
    ignored = zip_bytes([
        ("fixture/project/Unrelated.class", number("fixture/project/Unrelated", 73), ZIP_STORED),
    ])
    provided = zip_bytes([
        ("fixture/project/Provided.class", number("fixture/project/Provided", 47), ZIP_STORED),
    ])
    # The application archive intentionally contains no top-level class.
    (out / "boot.jar").write_bytes(zip_bytes([
        ("META-INF/MANIFEST.MF", b"Manifest-Version: 1.0\r\n\r\n", ZIP_STORED),
        ("BOOT-INF/lib/dependency.jar", library, ZIP_STORED),
        ("assets/extra.jar", ignored, ZIP_STORED),
        ("BOOT-INF/lib/bad.jar", b"not an archive", ZIP_STORED),
    ]))
    (out / "libraries.war").write_bytes(zip_bytes([
        ("WEB-INF/lib/dependency.jar", library, ZIP_DEFLATED),
        ("WEB-INF/lib-provided/container.jar", provided, ZIP_STORED),
        ("WEB-INF/assets/extra.jar", ignored, ZIP_STORED),
    ]))
    (out / "resource-part.apk").write_bytes(zip_bytes([
        ("AndroidManifest.xml", b"", ZIP_STORED),
        ("META-INF/MANIFEST.MF", b"Manifest-Version: 1.0\r\n\r\n", ZIP_STORED),
    ]))


def description(out, name, archives, release=None):
    project = {"archives": archives}
    if release is not None:
        project["java_version"] = release
    (out / (name + ".diana-jvm.json")).write_text(json.dumps(project), encoding="utf-8")


def project(out):
    nested(out)
    (out / "own").mkdir()
    (out / "resources.jar").write_bytes(zip_bytes([
        ("readme.txt", b"Project documentation\n", ZIP_STORED),
    ]))
    (out / "own" / "application.jar").write_bytes(zip_bytes([
        ("fixture/project/Caller.class", caller(), ZIP_DEFLATED),
    ]))
    (out / "unrelated.txt").write_text("No JVM file header.", encoding="utf-8")
    description(out, "project", [
        "own/application.jar", "libraries.war", "missing.jar", "resources.jar", "unrelated.txt",
        "../outside.jar", "own//application.jar",
    ])


def multi_release(out):
    def records(flag, duplicate=False):
        entries = [
            ("META-INF/MANIFEST.MF", flag, ZIP_STORED),
            ("fixture/project/Release.class", number("fixture/project/Release", 8), ZIP_STORED),
            ("fixture/project/Library.class", number("fixture/project/Library", 37), ZIP_STORED),
            ("META-INF/versions/9/fixture/project/Release.class",
             number("fixture/project/Release", 9), ZIP_STORED),
            ("META-INF/versions/17/fixture/project/Release.class",
             number("fixture/project/Release", 17), ZIP_DEFLATED),
            ("META-INF/versions/21/fixture/project/Release.class",
             number("fixture/project/Release", 21), ZIP_STORED),
            ("META-INF/versions/017/fixture/project/Library.class",
             number("fixture/project/Library", 65), ZIP_STORED),
            ("META-INF/versions/17/META-INF/Hidden.class",
             number("fixture/project/Hidden", 97), ZIP_STORED),
        ]
        if duplicate:
            entries.append(entries[4])
        return entries

    flag = b"Manifest-Version: 1.0\r\nMuLtI-ReLeAsE: TrUe\r\n\r\n"
    canonical = zip_bytes(records(flag))
    with ZipFile(io.BytesIO(canonical)) as archive:
        assert len(archive.infolist()) == 8
        assert archive.read("META-INF/MANIFEST.MF") == flag
    (out / "versioned.jar").write_bytes(canonical)
    (out / "unversioned.jar").write_bytes(zip_bytes(records(
        b"Manifest-Version: 1.0\n\nName: package/\nMulti-Release: true\n",
    )))
    (out / "duplicated.jar").write_bytes(zip_bytes(records(flag, duplicate=True)))
    (out / "folded.jar").write_bytes(zip_bytes(records(
        b"Manifest-Version: 1.0\r\nMulti-Release: tr\r\n Ue\r\n\r\n",
    )))
    bad = canonical.replace(b"ReLeAsE: Tr", b"ReLeAsE: fa", 1)
    assert len(bad) == len(canonical) and bad != canonical
    (out / "bad-manifest.jar").write_bytes(bad)
    for release in [8, 11, 17, 25]:
        description(out, f"java-{release}", ["versioned.jar"], release)
    description(out, "no-selection", ["versioned.jar"])
    description(out, "unversioned", ["unversioned.jar"], 21)
    description(out, "duplicated", ["duplicated.jar"], 17)
    description(out, "folded", ["folded.jar"], 17)
    description(out, "bad-manifest", ["bad-manifest.jar"], 17)


if __name__ == "__main__":
    action, output = sys.argv[1:]
    out = Path(output)
    out.mkdir(parents=True, exist_ok=True)
    {"project": project, "nested": nested, "multi-release": multi_release}[action](out)
