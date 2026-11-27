"""Generate independently known class and resource entries for JVM export."""

import io
from pathlib import Path
import subprocess
import sys
import zipfile

fixtures = Path(__file__).resolve().parent
output = Path(sys.argv[1]).resolve()
output.mkdir(parents=True, exist_ok=True)
base = output / "base"
variant = output / "variant"
variant.mkdir(exist_ok=True)

sources = [fixtures / "Edition.java", fixtures / "Helper.java"]
compiler = [
    "java", "-m", "jdk.compiler/com.sun.tools.javac.Main",
    "-source", "17", "-target", "17", "-Xlint:-options", "-g", "-parameters",
]
subprocess.run(
    [*compiler, "-d", base, *sources],
    check=True,
)
alternate = variant / "Edition.java"
alternate.write_text(sources[0].read_text().replace("input * 2 + 1", "input * 3 + 7"))
subprocess.run(
    [*compiler, "-d", variant, alternate],
    check=True,
)

edition = Path("fixture/exported/Edition.class")
helper = Path("fixture/exported/Helper.class")
manifest = b"Manifest-Version: 1.0\r\nMulti-Release: true\r\n\r\n"
nested = io.BytesIO()
with zipfile.ZipFile(nested, "w", zipfile.ZIP_DEFLATED) as jar:
    jar.writestr(helper.as_posix(), (base / helper).read_bytes())
    jar.writestr("assets/nested.bin", bytes(range(256)) * 1024 + b"nested")
    jar.writestr("META-INF/services/fixture.Service", b"fixture.exported.Helper\n")

for partial, filename in [(False, "complete.jar"), (True, "partial.jar")]:
    with zipfile.ZipFile(output / filename, "w", zipfile.ZIP_DEFLATED) as jar:
        jar.writestr("META-INF/MANIFEST.MF", manifest)
        jar.writestr("entry-name-does-not-match.class", (base / edition).read_bytes())
        jar.writestr(f"META-INF/versions/17/{edition.as_posix()}", (variant / edition).read_bytes())
        jar.writestr("fabric.mod.json", b'{"jars":[{"file":"dependencies/included.jar"}]}')
        jar.writestr("dependencies/included.jar", nested.getvalue())
        jar.writestr("native/linux/libfixture.so", b"\x7fELF\x00fixture", zipfile.ZIP_STORED)
        if partial:
            jar.writestr("broken.class", b"\xca\xfe\xba\xbe")
            jar.writestr("../sentinel.bin", b"alternate path")
            jar.writestr("assets/damaged.bin", b"original payload", zipfile.ZIP_STORED)
    if partial:
        path = output / filename
        data = path.read_bytes()
        assert data.count(b"original payload") == 1
        path.write_bytes(data.replace(b"original payload", b"modified payload", 1))

with zipfile.ZipFile(output / "extras.jar", "w") as jar:
    jar.writestr("assets/secondary.bin", b"second artifact")

with zipfile.ZipFile(output / "runtime.jar", "w", zipfile.ZIP_DEFLATED) as jar:
    jar.writestr("META-INF/MANIFEST.MF", manifest)
    jar.writestr(edition.as_posix(), (base / edition).read_bytes())
    jar.writestr(f"META-INF/versions/17/{edition.as_posix()}", (variant / edition).read_bytes())

with zipfile.ZipFile(output / "folded.jar", "w") as jar:
    jar.writestr("META-INF/MANIFEST.MF", b"Manifest-Version: 1.0\r\nX-Label: \xe2\x82\r\n \xac\r\n\r\n")
