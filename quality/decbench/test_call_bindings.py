"""Exercise original invoke decoding, attributed overloads and uncertain alignment."""

from pathlib import Path
import shutil
import subprocess
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import patch

import call_bindings
from dex_calls import read_calls

FIXTURES = Path(__file__).resolve().parents[1] / "fixtures"
SCALARS = FIXTURES / "atoms/scalar-use-views"
SCALAR_OWNER = "Lfixture/ScalarViews;"
SCALAR_CLASS = "e0:l0:c0"
ARRAY_AT, CARRIER_AT = f"{SCALAR_CLASS}:m0", f"{SCALAR_CLASS}:m2"
ARRAY = SCALAR_OWNER + "->array([C)Ljava/lang/String;"
CARRIER = SCALAR_OWNER + "->carrier(Ljava/lang/String;)I"


def manifest(location, owner, methods):
    return {
        "call_bindings": {"class_at": location, "class_member": owner},
        "methods": [{"at": at, "member": member, "call_bindings": True} for at, member in methods],
    }


class OriginalInvokesTest(unittest.TestCase):
    def test_dex_reads_per_site_identity_and_preserves_use_order(self):
        calls = read_calls(SCALARS / "classes.dex", CARRIER_AT, CARRIER)
        self.assertEqual([(item["name"], item["descriptor"]) for item in calls], [
            ("charAt", "(I)C"), ("probe", "(C)I"), ("probe", "(I)I"),
        ])
        self.assertEqual([item["pc"] for item in calls], [1, 5, 9])
        self.assertEqual([item["dispatch"] for item in calls], ["instance", "static", "static"])

    def test_symbolic_owner_difference_keeps_inheritance_uncertain(self):
        item = {"pc": 1, "name": "f", "descriptor": "()I", "dispatch": "instance"}
        original = dict(item, owner="Lfixture/Derived;")
        recovered = dict(item, owner="Lfixture/Base;", detail="")
        self.assertEqual(call_bindings.compare_call(original, recovered)["status"], "UNCHECKED")
        recovered["dispatch"] = "super"
        self.assertEqual(call_bindings.compare_call(original, recovered)["status"], "FAIL")
        original["dispatch"] = "special"
        self.assertEqual(call_bindings.compare_call(original, recovered)["status"], "UNCHECKED")


@unittest.skipUnless(shutil.which("java"), "needs a JDK")
class AttributedInvokesTest(unittest.TestCase):
    def scalar_checks(self, source):
        publication = {
            "exit_code": 0,
            "publication": {"schema_version": 5, "command": "get-class",
                            "selection": {"class_at": SCALAR_CLASS + "::" + SCALAR_OWNER},
                            "result": {"text": source}},
        }
        config = manifest(SCALAR_CLASS, SCALAR_OWNER, [(ARRAY_AT, ARRAY), (CARRIER_AT, CARRIER)])
        with patch.object(call_bindings, "run_cli", return_value=publication):
            return call_bindings.compare(Path("diana"), SCALARS / "classes.dex", config, 15)

    def test_single_wrong_overload_is_detected_at_its_original_pc(self):
        source = (SCALARS / "Case.java").read_text(encoding="utf-8")
        outputs = self.scalar_checks(source)
        for checks in outputs.values():
            self.assertTrue(checks)
            self.assertEqual({check["status"] for check in checks}, {"PASS"})
        mistaken = source.replace("probe((int) unit)", "probe(unit)")
        folded = mistaken.replace("int character = probe(unit);\n        int word = probe(unit);\n"
                                  "        return character + word;", "return probe(unit) + probe(unit);")
        checks = self.scalar_checks(folded)[CARRIER_AT]
        self.assertEqual([row["status"] for row in checks], ["PASS", "PASS", "FAIL"])
        self.assertEqual(checks[2]["id"], "invoke-9")
        self.assertEqual(checks[2]["expected"]["descriptor"], "(I)I")
        self.assertEqual(checks[2]["observed"]["descriptor"], "(C)I")
        self.assertEqual(checks[1]["observed"]["line"], checks[2]["observed"]["line"])
        self.assertLess(checks[1]["observed"]["column"], checks[2]["observed"]["column"])

    def test_unresolved_calls_and_missing_anchors_do_not_guess(self):
        source = (SCALARS / "Case.java").read_text(encoding="utf-8")
        unresolved = source.replace("probe((int) unit)", "probe(missing)")
        checks = self.scalar_checks(unresolved)
        self.assertEqual([row["status"] for row in checks[CARRIER_AT]], ["PASS", "PASS", "UNCHECKED"])
        self.assertEqual({row["status"] for row in checks[ARRAY_AT]}, {"PASS"})
        missing = source.replace("probe((int) unit)", "7")
        coverage = self.scalar_checks(missing)[CARRIER_AT]
        self.assertEqual([row["status"] for row in coverage], ["UNCHECKED"])
        self.assertIn("not uniquely aligned", coverage[0]["detail"])

    def test_jvm_uses_original_class_and_method_and_rejects_branch_paths(self):
        source = FIXTURES / "jvm/source-recovery/SourceRecovery.java"
        owner = "Lfixture/jvm/SourceRecovery;"
        calculate = owner + "->calculate(I)I"
        choose = owner + "->choose(I)I"
        with TemporaryDirectory() as scratch:
            work = Path(scratch)
            subprocess.run(["java", "-m", "jdk.compiler/com.sun.tools.javac.Main", "-d", str(work), str(source)],
                           check=True, capture_output=True, timeout=15)
            artifact = work / "fixture/jvm/SourceRecovery.class"
            methods = [{"at": "j0:m1", "member": calculate}, {"at": "j0:m3", "member": choose}]
            before = call_bindings.originals(artifact, "j0", owner, methods, work, 15)
            candidates = call_bindings.attribute(source.read_text(encoding="utf-8"), owner, [calculate], work, 15)
        results = call_bindings.comparisons(before["j0:m1"], candidates[calculate])
        self.assertEqual([(row["status"], row["expected"]["name"]) for row in results], [("PASS", "twice")])
        self.assertEqual(before["j0:m3"]["status"], "UNCHECKED")
        self.assertIn("ifle", before["j0:m3"]["detail"])

    def test_javac_recognizes_super_receiver_dispatch(self):
        source = FIXTURES / "jvm/relations/Relations.java"
        owner = "Lfixture/relations/Leaf;"
        member = owner + "->transform(I)I"
        text = source.read_text(encoding="utf-8")
        with TemporaryDirectory() as scratch:
            work = Path(scratch)
            base = call_bindings.attribute(text, "Lfixture/relations/Relations;", [member], work, 15)
            virtual = call_bindings.attribute(text.replace("super.transform", "this.transform"),
                                              "Lfixture/relations/Relations;", [member], work, 15)
        self.assertEqual(base[member][0]["dispatch"], "super")
        self.assertEqual(base[member][0]["owner"], "Lfixture/relations/Middle;")
        self.assertEqual(virtual[member][0]["dispatch"], "instance")
