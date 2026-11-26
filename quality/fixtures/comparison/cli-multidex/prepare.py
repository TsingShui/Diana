"""Assemble six existing DEX fixtures into a root-multidex APK."""

from pathlib import Path
import sys
from zipfile import ZIP_DEFLATED, ZipFile


def assemble(output):
    atoms = Path(__file__).resolve().parents[2] / "atoms"
    roots = (
        "array-basic",
        "expr-arith",
        "expr-compare",
        "expr-constants",
        "expr-wide",
        "field-basic",
    )
    with ZipFile(output, "w", compression=ZIP_DEFLATED) as archive:
        for number, fixture in enumerate(roots, start=1):
            entry = "classes.dex" if number == 1 else f"classes{number}.dex"
            archive.write(atoms / fixture / "classes.dex", entry)


if __name__ == "__main__":
    assemble(Path(sys.argv[1]))
