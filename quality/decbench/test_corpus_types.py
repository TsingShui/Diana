import os
from pathlib import Path
import unittest

import corpus_types
from dex_truth import methods


FIXTURES = Path(__file__).resolve().parents[1] / "fixtures/atoms"
ARTIFACT = FIXTURES / "scalar-use-views/classes.dex"
ARRAY_ARTIFACT = FIXTURES / "array-basic/classes.dex"


def check(case, text, **publication):
    return {receipt["id"]: receipt for receipt in corpus_types.compare(case, {"text": text, **publication})}


class DeclarationTypesTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.cases = {member.partition("->")[2].partition("(")[0]:
                     {"at": at, "member": member, "original_verified": True} for at, member in methods(ARTIFACT)}

    def test_verified_raw_char_parameter_and_int_return(self):
        case = self.cases["parameter"]
        correct = check(case, "package fixture; class ScalarViews { int parameter(char c) { return c; } }")
        wrong = check(case, "package fixture; class ScalarViews { long parameter(int c) { return c; } }")
        self.assertEqual({key: item["status"] for key, item in correct.items()},
                         {"return-type": "PASS", "parameter-1-type": "PASS"})
        self.assertEqual(wrong["return-type"]["expected"], {"slot": "return", "descriptor": "I"})
        self.assertEqual(wrong["return-type"]["observed"], {"slot": "return", "descriptor": "J"})
        self.assertEqual(wrong["return-type"]["status"], "FAIL")
        self.assertEqual(wrong["parameter-1-type"]["expected"], {"slot": "parameter-1", "descriptor": "C"})
        self.assertEqual(wrong["parameter-1-type"]["status"], "FAIL")

    def test_array_rank_and_spelling(self):
        case = self.cases["array"]
        for parameter in ("char[] chars", "char chars[]", "char @Dimension [] chars", "final char... chars"):
            with self.subTest(parameter=parameter):
                receipts = check(case, f"String array({parameter}) {{ return null; }}")
                self.assertEqual(set(receipts), {"parameter-1-type"})
                self.assertEqual(receipts["parameter-1-type"]["status"], "PASS")
                self.assertEqual(receipts["parameter-1-type"]["observed"]["descriptor"], "[C")
        wrong = check(case, "String array(char[][] chars) { return null; }")
        self.assertEqual(wrong["parameter-1-type"]["status"], "FAIL")
        self.assertEqual(wrong["parameter-1-type"]["observed"]["descriptor"], "[[C")
        vararg = check(case, "String array(char[]... chars) { return null; }")
        self.assertEqual(vararg["parameter-1-type"]["observed"]["descriptor"], "[[C")
        original_scalar = self.cases["boundaries"]
        self.assertEqual(check(original_scalar, "int boundaries()[] { return null; }")["return-type"]["observed"],
                         {"slot": "return", "descriptor": "[I"})
        at, member = next((at, member) for at, member in methods(ARRAY_ARTIFACT) if "->bytes()" in member)
        array_result = {"at": at, "member": member, "original_verified": True}
        self.assertEqual(check(array_result, "byte[] bytes() { return null; }")["return-type"]["status"], "PASS")
        scalar = check(array_result, "byte bytes() { return 0; }")["return-type"]
        self.assertEqual(scalar["expected"], {"slot": "return", "descriptor": "[B"})
        self.assertEqual(scalar["status"], "FAIL")

    def test_opaque_slot_does_not_discard_known_sibling_slot(self):
        case = self.cases["parameter"]
        report = check(case, "int parameter(Object boxed) { return 0; }")
        self.assertEqual(report["return-type"]["status"], "PASS")
        self.assertEqual(report["parameter-1-type"]["status"], "UNCHECKED")
        self.assertEqual(check(case, "? parameter(char c) { return 0; }")["parameter-1-type"]["status"], "PASS")
        self.assertEqual(check(case, "int parameter(char c);", error="body recovery incomplete")["return-type"]["status"],
                         "PASS")
        generic = "public static <T extends Comparable<T>> int parameter(char c) { return c; }"
        self.assertEqual(check(case, generic)["return-type"]["status"], "PASS")

    def test_rejects_sibling_overloads_invocations_and_inexact_owner(self):
        case = self.cases["parameter"]
        for text in (
            "package fixture; class ScalarViews { int parameter(char c); int parameter(int i) { return i; } }",
            "package fixture; class ScalarViews { <T> int parameter(char c); int parameter(int i) { return i; } }",
            "package unrelated; class ScalarViews { int parameter(char c) { return c; } }",
            "package fixture; class Other { int parameter(char c) { return c; } }",
            "package fixture; class ScalarViews { class Inner { int parameter(char c) { return c; } } }",
            "package fixture; class ScalarViews { int x = parameter(0); int other() { return parameter(1); } }",
            "return parameter(1);", "int parameter(char c) { return c; } int other() { return 0; }",
            "int parameter(char c, char d) { return c + d; }",
        ):
            with self.subTest(text=text):
                self.assertEqual({row["status"] for row in check(case, text).values()}, {"UNCHECKED"})
        separate_arity = "package fixture; class ScalarViews { int parameter(); int parameter(char c) {return c;} }"
        self.assertEqual(check(case, separate_arity)["parameter-1-type"]["status"], "PASS")

    def test_original_must_be_verified_before_types_are_checked(self):
        case = dict(self.cases["parameter"], original_verified=False, original_error="selected original is absent")
        rows = check(case, "int parameter(int i) { return i; }")
        self.assertEqual({row["status"] for row in rows.values()}, {"SETUP_ERROR"})
        self.assertEqual({row["detail"] for row in rows.values()}, {"selected original is absent"})

    @unittest.skipUnless(os.environ.get("DECBENCH_DIANA"), "set DECBENCH_DIANA for a real Diana publication")
    def test_diana_scalar_fixture_publishes_original_slots(self):
        from evidence import method

        binary = Path(os.environ["DECBENCH_DIANA"])
        for selected in ("parameter", "array", "boundaries"):
            case = self.cases[selected]
            publication = method(binary, ARTIFACT, case["at"], 15)
            receipts = corpus_types.compare(case, publication)
            self.assertTrue(receipts)
            self.assertEqual({receipt["status"] for receipt in receipts}, {"PASS"})


if __name__ == "__main__":
    unittest.main()
