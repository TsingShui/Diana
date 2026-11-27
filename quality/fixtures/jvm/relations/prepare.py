"""Compile the fixture and preserve a fixed physical archive layout."""

from io import BytesIO
import json
from pathlib import Path
import subprocess
import sys
from zipfile import ZIP_DEFLATED, ZipFile


def prepare(destination):
    destination.mkdir(parents=True, exist_ok=True)
    classes = destination / "classes"
    subprocess.run(
        [
            "java", "-m", "jdk.compiler/com.sun.tools.javac.Main",
            "-source", "17", "-target", "17", "-Xlint:-options", "-d", str(classes),
            str(Path(__file__).with_name("Relations.java")),
            str(Path(__file__).with_name("Diamonds.java")),
            str(Path(__file__).with_name("Check.java")),
        ],
        check=True,
    )
    subprocess.run(
        ["java", "-cp", str(classes), "fixture.relations.Check"],
        check=True,
    )
    generated = classes / "fixture/relations"

    def entry(archive, name):
        archive.writestr(
            f"fixture/relations/{name}.class",
            (generated / f"{name}.class").read_bytes(),
        )

    nested = BytesIO()
    with ZipFile(nested, "w", ZIP_DEFLATED) as archive:
        entry(archive, "Leaf")
    for variant in ("relations", "duplicate", "damaged"):
        with ZipFile(destination / f"{variant}.jar", "x", ZIP_DEFLATED) as archive:
            archive.writestr("fabric.mod.json", json.dumps({"jars": [{"file": "nested/leaf.jar"}]}))
            for name in ("Relations", "Signal", "Child", "Base", "Middle", "Terminal"):
                entry(archive, name)
            archive.writestr("nested/leaf.jar", nested.getvalue())
            if variant == "duplicate":
                archive.writestr("second/Terminal.class", (generated / "Terminal.class").read_bytes())
            if variant == "damaged":
                archive.writestr("unreadable.class", bytes.fromhex("ca fe ba be 00"))

    with ZipFile(destination / "diamond.jar", "x", ZIP_DEFLATED) as archive:
        for name in ("Diamonds", "Common", "Left", "Right", "Tip", "Winning"):
            entry(archive, name)


if __name__ == "__main__":
    prepare(Path(sys.argv[1]))
