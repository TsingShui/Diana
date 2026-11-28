"""Exercise the offline structural projection independently of behavior checks."""

import shutil
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import patch

import flow_shape


OWNER = "Lfixture/Example;"


class FlowShapeTest(unittest.TestCase):
    def graphs(self, reference, candidate, names=("subject",)):
        if not shutil.which("java"):
            self.skipTest("JDK Java launcher is unavailable")
        methods = [{"member": OWNER + "->" + name + "(I)I"} for name in names]
        with TemporaryDirectory(prefix="test-decbench-flow-") as scratch:
            paths = [Path(scratch, name) for name in ("Reference.java", "Observed.java")]
            for path, text in zip(paths, (reference, candidate)):
                path.write_text("package fixture; class Example {" + text + "}", encoding="utf-8")
            return flow_shape.parse_graphs(*paths, OWNER, methods, timeout=30)

    def test_equal_behavior_with_duplicate_switch_returns_changes_structure(self):
        reference, candidate = self.graphs(
            "int subject(int x) { switch (x) { case 1: case 2: return 9; default: return x; }}",
            "int subject(int x) { switch (x) { case 1: return 9; case 2: return 9; default: return x; }}",
        )
        before, after = reference[0]["graph"], candidate[0]["graph"]
        self.assertFalse(flow_shape.equal_graphs(before, after))
        self.assertEqual(
            flow_shape.divergence(before, after),
            "authored case 1 and case 2 share one entry, while the recovered entries are distinct",
        )

    def test_condition_polarity_and_lexical_block_boundaries_are_transparent(self):
        reference, candidate = self.graphs(
            "int subject(int x) { if (x > 0) { trace(); } else { return x; } a(); b(); return 9; }",
            "int subject(int arg) { if (arg <= 0) { return arg; } trace(); { a(); } b(); return 9; }",
        )
        # The recovered branch moves trace onto the surviving path. Compare
        # against a reference with that same maximal block after normalization.
        self.assertTrue(flow_shape.equal_graphs(reference[0]["graph"], candidate[0]["graph"]))

    def test_loop_backedge_and_break_are_preserved(self):
        reference, candidate = self.graphs(
            "int subject(int x) { while (x > 0) { if (done()) break; x--; } return x; }",
            "int subject(int x) { if (x > 0) { if (!done()) x--; } return x; }",
        )
        before, after = reference[0]["graph"], candidate[0]["graph"]
        loop_index = next(index for index, node in enumerate(before) if node["kind"] == "while")
        self.assertTrue(any(index > loop_index and ["next", loop_index] in node["edges"]
                            for index, node in enumerate(before)))
        self.assertFalse(flow_shape.equal_graphs(before, after))

    def test_unsupported_statement_and_syntax_error_stay_method_local(self):
        reference, candidate = self.graphs(
            "int subject(int x) { return x; } int other(int x) { return x; }",
            "int subject(int x) { if () return x; return x; } "
            "int other(int x) { try { return x; } finally { trace(); } } "
            "int intact(int x) { return x; }",
            ("subject", "other", "intact"),
        )
        self.assertIn("syntax", candidate[0]["detail"])
        self.assertIn("TRY", candidate[1]["detail"])
        self.assertIn("graph", candidate[2])
        self.assertIn("graph", reference[0])

    def test_ambiguous_overloads_are_unchecked(self):
        _, candidate = self.graphs(
            "int subject(int x) { return x; }",
            "int subject(int x) { return x; } int subject(String x) { return 0; }",
        )
        self.assertIn("ambiguous", candidate[0]["detail"])

    def test_publication_must_address_the_requested_original_class(self):
        with TemporaryDirectory() as scratch:
            reference = Path(scratch, "Reference.java")
            reference.write_text("class Example {}", encoding="utf-8")
            manifest = {
                "manifest": Path(scratch, "decbench.json"),
                "structure": {"reference": reference.name, "class_at": "j1", "class_member": OWNER},
                "methods": [{"at": "j1:m0", "member": OWNER + "->subject(I)I", "structure": True}],
            }
            external = {
                "exit_code": 0,
                "publication": {"schema_version": 5, "command": "get-class",
                                "selection": {"class_at": "j2", "class": OWNER},
                                "result": {"text": "class Example {}"}},
            }
            with patch.object(flow_shape, "run_cli", return_value=external):
                outcome = flow_shape.compare(Path("diana"), Path("input.jar"), manifest, timeout=30)
            self.assertEqual(outcome["j1:m0"][0]["status"], "UNCHECKED")
