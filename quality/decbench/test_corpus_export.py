"""Check attribution and local gaps in an ordinary Diana export."""

import json
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest

import corpus_export


class ExportTest(unittest.TestCase):
    def setUp(self):
        self.root = Path(self.enterContext(TemporaryDirectory())).resolve()

    def test_full_signature_selects_only_the_exact_overload_and_owner(self):
        path = self.root / "Probe.java"
        path.write_text("""package sample;
import alpha.Input;
class Probe {
    int choose(Input value) { return 1; }
    int choose(beta.Input value) { return 2; }
    boolean choose(int value) { return false; }
    int missing(int value);
    class Inner { int choose(int value) { return 9; } }
}
""")
        owner = "Lsample/Probe;"
        methods = [(str(index), owner + "->" + member) for index, member in enumerate((
            "choose(Lalpha/Input;)I", "choose(Lbeta/Input;)I", "choose(Lother/Input;)I",
            "choose(I)I", "choose(I)Z", "missing(I)I",
        ))]
        diagnostic = {"class": owner, "name": "missing", "prototype": "(I)I", "stage": "recovery"}
        result = corpus_export.collect(self.root, path, owner, methods, {owner}, [diagnostic])
        self.assertIn("return 1;", result["0"]["text"])
        self.assertIn("return 2;", result["1"]["text"])
        self.assertIn("error", result["2"])
        self.assertIn("error", result["3"])
        self.assertIn("return false;", result["4"]["text"])
        self.assertEqual(result["4"]["issues"], [])
        self.assertFalse(result["5"]["has_body"])
        self.assertEqual(result["5"]["issues"], [diagnostic])

    def test_original_generated_import_keeps_other_type_attribution_available(self):
        path = self.root / "Probe.java"
        owner = "Lsample/Probe;"
        methods = [("selected", owner + "->choose(J)Ljava/lang/String;")]
        generated = "import generated.Companion$-CC;"
        generated_type = "Lgenerated/Companion$-CC;"
        for imports, owners, matched in (
            (generated, {generated_type}, True),
            (generated, set(), False),
            (generated + " import generated.*;", {generated_type}, False),
            (generated + " import other.String;", {generated_type}, False),
            (generated + " import java.lang.String; import other.String;", {generated_type}, False),
            (generated, {generated_type, "Lsample/String;"}, False),
            ("import static generated.Companion.String;", {"Lgenerated/Companion;"}, False),
            ("import generated.bad-name.String;", {"Lgenerated/bad-name/String;"}, False),
            ("import generated.Companion$ -CC;", {generated_type}, False),
        ):
            with self.subTest(imports=imports, owners=owners):
                path.write_text(f"""package sample;
{imports}
class Probe {{
    String choose(long value) {{ return format(value); }}
    String choose(long value, boolean flag) {{ return fallback(value, flag); }}
}}
""")
                selected = corpus_export.collect(self.root, path, owner, methods, owners | {owner}, [])["selected"]
                if matched:
                    self.assertEqual(selected["text"], "String choose(long value) { return format(value); }")
                else:
                    self.assertIn("error", selected)

    def test_fallback_coordinates_and_source_failure_leave_siblings_available(self):
        owner = "Lsample/Probe;"
        classes = {f"e4:l0:c{index}": owner for index in range(3)}
        path = "source-definitions/e4-l0-c1.java"
        report = self.root / "export-report.jsonl"
        fallback = {"kind": "class-path", "entry": 4, "logical": 0, "class_def": 1,
                    "descriptor_units": list(map(ord, owner)), "path": path}
        failure = {"kind": "source", "entry": 4, "logical": 0, "class_def": 2, "reason": "broken class"}
        report.write_text("\n".join(map(json.dumps, (fallback, failure))) + "\n")
        paths, _, failed = corpus_export.export_report(self.root, classes, True)
        self.assertEqual(corpus_export.class_paths(self.root, classes, paths, failed), {
            "e4:l0:c0": self.root / "sources/sample/Probe.java", "e4:l0:c1": self.root / path,
        })
        self.assertEqual(corpus_export.class_paths(self.root, classes, {}, set()), {})
        report.write_text(json.dumps({**fallback, "path": "../Probe.java"}) + "\n")
        with self.assertRaisesRegex(ValueError, "original class identity"):
            corpus_export.export_report(self.root, classes, True)


if __name__ == "__main__":
    unittest.main()
