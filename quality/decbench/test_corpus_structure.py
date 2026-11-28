"""Exercise the batched public source and recovered statement CFG projection."""

from hashlib import sha256
from io import BytesIO
from pathlib import Path
import shutil
import subprocess
from tarfile import TarInfo, open as open_tar
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import patch

import corpus_structure
import flow_shape


OWNER = "Lfixture/Example;"
MEMBER = OWNER + "->subject(I)I"
SOURCE_FILE = "app/src/main/java/fixture/Example.java"
SOURCE = b"""package fixture;
public class Example {
    int subject(int x) {
        while (x > 0) {
            x--;
        }
        return x;
    }
    int neighbor(int x) {
        return x;
    }
}
"""
RENDERED = """
package fixture;

class Example {
    static;
    int subject(String x);
    int neighbor(int x) { if () return x; return 0; }
    int subject(int x) { while (x > 0) { x--; } return x; }
}
"""


def make_case(locator, name, start, end, *, file=SOURCE_FILE, data=SOURCE):
    return {"at": locator, "member": OWNER + "->" + name + "(I)I", "source": {
        "path": file, "start_line": start, "end_line": end, "sha256": sha256(data).hexdigest(),
    }}


def sources(path):
    with open_tar(path, "w:gz") as output:
        entry = TarInfo("release/" + SOURCE_FILE)
        entry.size = len(SOURCE)
        output.addfile(entry, BytesIO(SOURCE))


class CorpusStructureTest(unittest.TestCase):
    @unittest.skipUnless(shutil.which("java"), "JDK Java launcher is unavailable")
    def test_one_process_keeps_missing_methods_and_unrelated_syntax_local(self):
        methods = [make_case("subject", "subject", 3, 8), make_case("neighbor", "neighbor", 9, 11)]
        with TemporaryDirectory() as directory:
            archive = Path(directory) / "source.tar.gz"
            sources(archive)
            with patch.object(flow_shape.subprocess, "run", wraps=subprocess.run) as java:
                result = corpus_structure.inspect(
                    methods, archive, "release",
                    {"subject": {"text": RENDERED}, "neighbor": {"error": "missing publication"}}, 15,
                )
        self.assertEqual(java.call_count, 1)
        self.assertIn("--batch", java.call_args.args[0])
        subject = result["subject"]
        self.assertEqual(subject["outcome"], "MATCH")
        self.assertIn("while", {node["kind"] for node in subject["observed"]})
        self.assertIsNone(result["neighbor"]["observed"])
        self.assertEqual(result["neighbor"]["outcome"], "UNCHECKED")
        self.assertIn("missing publication", result["neighbor"]["detail"])
        self.assertIsNotNone(result["neighbor"]["expected"])

    @unittest.skipUnless(shutil.which("java"), "JDK Java launcher is unavailable")
    def test_fragment_preserves_original_structure_and_partial_coverage(self):
        case = make_case("subject", "subject", 3, 8)
        fragment = "int subject(int x) { if (x > 0) { x--; } return x; }"
        with TemporaryDirectory() as directory:
            archive = Path(directory) / "source.tar.gz"
            sources(archive)
            report = corpus_structure.inspect([case], archive, "release",
                                              {"subject": {"text": fragment}}, 15, backend="jadx")
            partial = corpus_structure.inspect(
                [case], archive, "release",
                {"subject": {"text": fragment, "error": "partial peer publication"}}, 15, backend="jadx",
            )
        self.assertEqual(report["subject"]["outcome"], "DIFFERENT")
        self.assertIn("normalized statement control flow differs", report["subject"]["detail"])
        self.assertEqual(partial["subject"]["outcome"], "UNCHECKED")
        self.assertEqual(partial["subject"]["observed"], report["subject"]["observed"])

    @unittest.skipUnless(shutil.which("java"), "JDK Java launcher is unavailable")
    def test_owner_parameter_and_source_fingerprint_checks_limit_results(self):
        valid = make_case("subject", "subject", 3, 8)
        stale = make_case("neighbor", "neighbor", 9, 11, data=SOURCE + b"\n")
        wrong = RENDERED.replace("package fixture;", "package wrong;")
        with TemporaryDirectory() as directory:
            archive = Path(directory) / "source.tar.gz"
            sources(archive)
            result = corpus_structure.inspect(
                [valid, stale], archive, "release",
                {"subject": {"text": wrong}, "neighbor": {"text":
                    "package fixture; class Example { int neighbor(int x) { return x; } }"}},
                15,
            )
            other_overload = corpus_structure.inspect(
                [valid], archive, "release",
                {"subject": {"text":
                    "package fixture; class Example { int subject(int x); int subject(String x) { return 0; } }"}},
                15,
            )
        self.assertEqual(result["subject"]["outcome"], "UNCHECKED")
        self.assertIn("owner", result["subject"]["detail"])
        self.assertEqual(result["neighbor"]["outcome"], "SETUP_ERROR")
        self.assertIn("fingerprint", result["neighbor"]["detail"])
        self.assertIsNotNone(result["neighbor"]["observed"])
        self.assertEqual(other_overload["subject"]["outcome"], "UNCHECKED")
        self.assertIn("signature", other_overload["subject"]["detail"])

    @unittest.skipUnless(shutil.which("java"), "JDK Java launcher is unavailable")
    def test_syntax_errors_inside_the_selected_body_remain_unchecked(self):
        valid = make_case("subject", "subject", 3, 8)
        text = "package fixture; class Example { int subject(int x) { if () return x; return 0; } }"
        with TemporaryDirectory() as directory:
            archive = Path(directory) / "source.tar.gz"
            sources(archive)
            report = corpus_structure.inspect([valid], archive, "release", {"subject": {"text": text}}, 15)
        self.assertEqual(report["subject"]["outcome"], "UNCHECKED")
        self.assertIn("source syntax", report["subject"]["detail"])
        self.assertIsNotNone(report["subject"]["expected"])
        self.assertIsNone(report["subject"]["observed"])

    def test_complete_worker_output_survives_timeout(self):
        units = [flow_shape.ParseUnit(OWNER, MEMBER, b"source") for _ in range(3)]
        output = (b'0\t0\tgraph\t[{"kind":"entry","edges":[]}]\n'
                  b'1\t0\tunchecked\t"unavailable source"\n'
                  b'2\t0\tgraph\t[{"kind":"entry"')
        failure = subprocess.TimeoutExpired(["java"], 1, output=output)
        with patch.object(flow_shape.subprocess, "run", side_effect=failure):
            result = flow_shape.parse_batch(units, 1)
        self.assertEqual(result[0]["graph"], [{"kind": "entry", "edges": []}])
        self.assertEqual(result[1]["detail"], "unavailable source")
        self.assertEqual(result[2]["setup_error"], "structural probe batch timed out")

    def test_structural_worker_startup_failure_is_not_a_source_limitation(self):
        units = [flow_shape.ParseUnit(OWNER, MEMBER, b"source")]
        with patch.object(flow_shape.subprocess, "run",
                          return_value=subprocess.CompletedProcess(["java"], 127, b"", b"JDK broken")):
            result, = flow_shape.parse_batch(units, 1)
        self.assertEqual(result, {"setup_error": "structural probe worker exited 127"})

    def test_batches_satisfy_unit_and_source_byte_bounds(self):
        units = [(index, flow_shape.ParseUnit(OWNER, MEMBER, b"source")) for index in range(5)]
        with patch.object(corpus_structure, "BATCH_UNITS", 2), \
                patch.object(corpus_structure, "BATCH_BYTES", 8), \
                patch.object(corpus_structure, "parse_batch", side_effect=lambda rows, _: [{} for _ in rows]) as parse:
            results = list(corpus_structure.graph_batches(units, 1))
        self.assertEqual(results, [(index, {}) for index in range(5)])
        self.assertEqual([len(call.args[0]) for call in parse.call_args_list], [1, 1, 1, 1, 1])


if __name__ == "__main__":
    unittest.main()
