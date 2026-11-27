"""Compile the before and after Java sources to original JVM artifacts."""

from pathlib import Path
import subprocess
import sys
from zipfile import ZIP_DEFLATED, ZipFile


def prepare(destination):
    root = Path(__file__).resolve().parent
    destination.mkdir(parents=True, exist_ok=True)
    for side in ("before", "after"):
        outputs = destination / side
        subprocess.run([
            "java", "-m", "jdk.compiler/com.sun.tools.javac.Main",
            "-source", "17", "-target", "17", "-Xlint:-options", "-g", "-parameters",
            "-d", str(outputs), str(root / side / "Expansion.java"), str(root / "Verify.java"),
        ], check=True)
        with ZipFile(destination / f"{side}.jar", "w", compression=ZIP_DEFLATED) as jar:
            name = "fixture/inline/Expansion.class"
            jar.write(outputs / name, name)


if __name__ == "__main__":
    prepare(Path(sys.argv[1]))
