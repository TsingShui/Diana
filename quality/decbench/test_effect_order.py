"""Witness normalization and conservative coverage for authored effects."""

from pathlib import Path
import shutil
import subprocess
from tempfile import TemporaryDirectory
import unittest

from effect_order import compare_witnesses, parse, verify_jvm_reference


OWNER = "Lsample/Example;"


def source(methods):
    return f"""package sample;

public final class Example {{
    private int left;
    private int right;
    private int count;
    private String label;

    {methods}
}}
""".encode("utf-8")


@unittest.skipUnless(shutil.which("java"), "the effect witness requires a JDK")
class EffectOrderTest(unittest.TestCase):
    def test_compound_assignments_and_renamed_local_preserve_provenance(self):
        expected = source("""
            int update(int delta) {
                left += delta;
                int snapshot = left;
                count++;
                label = "value=" + snapshot;
                return count;
            }
        """)
        recovered = source("""
            int update(int diff) {
                this.left = this.left + diff;
                int saved = this.left;
                ++this.count;
                this.label = "value=" + (saved);
                return this.count;
            }
        """)
        before, after = parse(expected, recovered, OWNER, "Example", ["update(I)I"], 15)
        self.assertEqual(before, after)
        self.assertEqual(before[0][0], "witness")
        observations = before[0][1]
        store = observations[5]
        self.assertEqual(store["member"], OWNER + "->label:Ljava/lang/String;")
        self.assertEqual(store["value"]["origin"]["operands"][1]["origin"], {"read": 2})

    def test_detects_store_that_moves_before_snapshot_load(self):
        expected = source("""
            int exchange(int replacement) {
                int previous = left;
                left = replacement;
                right = previous;
                return right;
            }
        """)
        recovered = source("""
            int exchange(int next) {
                this.left = next;
                this.right = this.left;
                return this.right;
            }
        """)
        before, after = parse(expected, recovered, OWNER, "Example", ["exchange(I)I"], 15)
        self.assertEqual(before[0][0], "witness")
        self.assertEqual(after[0][0], "witness")
        result = compare_witnesses(before[0][1], after[0][1])
        self.assertEqual(result["status"], "FAIL")
        self.assertEqual(result["detail"],
                         f"event 1: authored read {OWNER}->left:I, recovered write {OWNER}->left:I")
        self.assertEqual(before[0][1][2]["value"]["origin"], {"read": 0})
        self.assertEqual(after[0][1][2]["value"]["origin"], {"read": 1})

    def test_same_effect_targets_with_changed_value_dependency(self):
        expected = source("""
            int select() {
                int earlier = left;
                int later = right;
                count = earlier;
                return later;
            }
        """)
        recovered = source("""
            int select() {
                int a = this.left;
                int b = this.right;
                this.count = b;
                return b;
            }
        """)
        before, after = parse(expected, recovered, OWNER, "Example", ["select()I"], 15)
        self.assertEqual(before[0][0], "witness")
        self.assertEqual(after[0][0], "witness")
        result = compare_witnesses(before[0][1], after[0][1])
        self.assertEqual(result["status"], "FAIL")
        self.assertEqual(result["detail"], f"event 3: changed value provenance for write {OWNER}->count:I")

    def test_int_arithmetic_can_prove_an_equivalent_value(self):
        expected = source("int update(int value) { left = value + 1; return left - 1; }")
        recovered = source("int update(int input) { this.left = input - -1; return input; }")
        before, after = parse(expected, recovered, OWNER, "Example", ["update(I)I"], 15)
        result = compare_witnesses(before[0][1], after[0][1])
        self.assertEqual(result["status"], "FAIL")
        self.assertEqual(result["detail"], f"event 2: authored read {OWNER}->left:I, recovered return")

        retained_read = source("int update(int input) { this.left = input - -1; return this.left + -1; }")
        before, after = parse(expected, retained_read, OWNER, "Example", ["update(I)I"], 15)
        self.assertEqual(compare_witnesses(before[0][1], after[0][1])["status"], "PASS")

    def test_unproved_and_interned_string_expressions_are_unchecked(self):
        expected = source("""
            String update(String value) { return "<" + value + ">"; }
            String folded() { return "hello" + (1 + 2); }
            String local() { final String a = "hello"; final String b = " world"; return a + b; }
        """)
        recovered = source("""
            String update(String value) { return "<" + (value + ">"); }
            String folded() { return "hello3"; }
            String local() { String a = "hello"; String b = " world"; return a + b; }
        """)
        before, after = parse(expected, recovered, OWNER, "Example", [
            "update(Ljava/lang/String;)Ljava/lang/String;", "folded()Ljava/lang/String;",
            "local()Ljava/lang/String;",
        ], 15)
        self.assertEqual(compare_witnesses(before[0][1], after[0][1])["status"], "UNCHECKED")
        self.assertIn("interned constant", before[1][1])
        self.assertIn("constant local", before[2][1])
        self.assertEqual(after[2][0], "witness")

    def test_prepared_jvm_bytes_must_match_the_authored_source(self):
        original = source("int update(int value) { return value; }")
        stale = source("int update(int value) { return value + 1; }")
        with TemporaryDirectory(prefix="decbench-effect-fixture-") as directory:
            scratch = Path(directory)
            file = scratch / "Example.java"
            file.write_bytes(original)
            subprocess.run([
                "java", "-m", "jdk.compiler/com.sun.tools.javac.Main",
                "-source", "17", "-target", "17", "-Xlint:-options", "-g", "-d", directory, str(file),
            ], check=True, timeout=15)
            bytecode = (scratch / "sample" / "Example.class").read_bytes()
        verify_jvm_reference(original, bytecode, OWNER, "debug", 15)
        with self.assertRaisesRegex(ValueError, "prepared class differs"):
            verify_jvm_reference(stale, bytecode, OWNER, "debug", 15)

    def test_unmodeled_call_branch_and_alias_keep_neighbor_evidence(self):
        same = source("""
            int update(int value) { left = value; return value; }
            int branch(int value) { if (value == 0) return 0; return value; }
            int alias(Example other) { return other.left; }
            int helper() { return Math.abs(left); }
            int missing() { return unknown; }
        """)
        before, after = parse(
            same, same, OWNER, "Example",
            ["update(I)I", "branch(I)I", "alias(Lsample/Example;)I", "helper()I", "missing()I"], 15,
        )
        self.assertEqual(before, after)
        self.assertEqual(before[0][0], "witness")
        for index, reason in enumerate(("IF", "receiver", "METHOD_INVOCATION", "attribution"), 1):
            self.assertEqual(before[index][0], "unchecked")
            self.assertIn(reason, before[index][1])

    def test_overload_and_parameter_order_are_identified_by_descriptor(self):
        same = source("""
            int update(int first, int second) { left = second; return first; }
            String update(String first, String second) { label = first; return second; }
        """)
        before, after = parse(
            same, same, OWNER, "Example",
            ["update(II)I", "update(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;"], 15,
        )
        self.assertEqual(before, after)
        self.assertEqual(before[0][1][0]["member"], OWNER + "->left:I")
        self.assertEqual(before[0][1][0]["value"]["origin"], {"parameter": 1})
        self.assertEqual(before[1][1][0]["member"], OWNER + "->label:Ljava/lang/String;")
        self.assertEqual(before[1][1][0]["value"]["origin"], {"parameter": 0})


if __name__ == "__main__":
    unittest.main()
