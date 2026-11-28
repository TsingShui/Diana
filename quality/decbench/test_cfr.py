from contextlib import redirect_stderr, redirect_stdout
import io
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from cfr import create_launcher, file_digest, parse_inventory, serve


JAVAP_SAMPLE = """Compiled from "Example.java"
public final class fixture.jvm.Example {
  private int trace;
    descriptor: I
  public fixture.jvm.Example();
    descriptor: ()V

  static {};
    descriptor: ()V

  public int overloaded(int[], int);
    descriptor: ([II)I

  public int overloaded(java.lang.String);
    descriptor: (Ljava/lang/String;)I
}
"""


class CfrTest(unittest.TestCase):
    def test_inventory_retains_original_method_ordinals_and_overloads(self):
        owner, methods = parse_inventory(JAVAP_SAMPLE)
        self.assertEqual(owner, "Lfixture/jvm/Example;")
        self.assertEqual(methods, [
            {"at": "j0:m0", "member": "Lfixture/jvm/Example;-><init>()V"},
            {"at": "j0:m1", "member": "Lfixture/jvm/Example;-><clinit>()V"},
            {"at": "j0:m2", "member": "Lfixture/jvm/Example;->overloaded([II)I"},
            {"at": "j0:m3", "member": "Lfixture/jvm/Example;->overloaded(Ljava/lang/String;)I"},
        ])

    def test_unrecognized_inventory_is_unavailable(self):
        with self.assertRaisesRegex(ValueError, "did not identify an original class"):
            parse_inventory("Error: class not found")

    def test_class_observation_is_raw_cfr_publication(self):
        source = "/* Decompiled with CFR */\npublic final class Example {}\n"
        with (
            tempfile.TemporaryDirectory() as directory,
            patch("cfr.run_java", side_effect=[JAVAP_SAMPLE, source]) as java,
            redirect_stdout(io.StringIO()) as stdout,
        ):
            artifact = Path(directory) / "Example.class"
            artifact.touch()
            jar = Path(directory) / "cfr.jar"
            jar.touch()
            result = serve(jar, file_digest(jar), ["get-class", str(artifact), "j0", "--format", "json"], 7)
        self.assertEqual(result, 0)
        response = json.loads(stdout.getvalue())
        self.assertEqual(response["selection"]["class"], "Lfixture/jvm/Example;")
        self.assertEqual(response["result"], {"text": source})
        self.assertEqual(java.call_args.args, (["java", "-jar", str(jar), str(artifact)], 7))

    def test_selected_method_is_reported_unmeasured(self):
        with tempfile.TemporaryDirectory() as directory, redirect_stderr(io.StringIO()) as stderr:
            artifact = Path(directory) / "Example.class"
            artifact.touch()
            result = serve(Path("unused"), "", ["get-method", str(artifact), "j0:m1", "--format", "json"], 7)
        self.assertEqual(result, 2)
        self.assertIn("method publication is unmeasured", stderr.getvalue())

    @unittest.skipUnless(os.name == "posix", "POSIX executable launcher")
    def test_launcher_detects_changed_jar_and_preserves_existing_launcher(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            artifact, jar, launcher = (root / name for name in ("Example.class", "cfr.jar", "run-cfr"))
            artifact.touch()
            jar.write_bytes(b"original")
            create_launcher(jar, launcher)
            original = launcher.read_bytes()
            jar.write_bytes(b"changed")
            with self.assertRaises(FileExistsError):
                create_launcher(jar, launcher)
            self.assertEqual(launcher.read_bytes(), original)
            result = subprocess.run(
                [str(launcher), "methods", str(artifact), "--format", "json", "--no-limit"],
                capture_output=True, text=True, timeout=5, check=False,
            )
        self.assertEqual(result.returncode, 2)
        self.assertIn("CFR JAR has changed", result.stderr)


if __name__ == "__main__":
    unittest.main()
