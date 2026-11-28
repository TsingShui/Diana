from hashlib import sha256
from io import BytesIO
from pathlib import Path
import tarfile
from tempfile import TemporaryDirectory
import unittest
from zipfile import ZipFile

import corpus_coverage
from dex_truth import apk_methods
from evidence import file_digest


FIXTURE = Path(__file__).resolve().parents[1] / "fixtures/atoms/scalar-use-views/classes.dex"
SOURCE_PATH = "app/src/main/java/example/Fixture.java"
SOURCE_ROOT = "fixture-release"


class CorpusCoverageTests(unittest.TestCase):
    def setUp(self):
        temporary = TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        path = Path(temporary.name)
        self.apk, self.source = path / "release.apk", path / "source.tar.gz"
        with ZipFile(self.apk, "w") as archive:
            archive.writestr("README", b"Fixture")
            archive.write(FIXTURE, "classes.dex")
            archive.write(FIXTURE, "classes2.dex")
            archive.write(FIXTURE, "assets/classes.dex")
        self.methods = apk_methods(self.apk)
        self.write_archive([SOURCE_PATH, "app/src/main/kotlin/example/Fixture.kt", "app/src/test/java/Foo.java",
                            "build-support/script.kt", "Duplicate.java", "Duplicate.java", "../Unsafe.java"])
        self.source_pin = {"path": SOURCE_PATH, "sha256": sha256(b"class Fixture {}\n").hexdigest(),
                           "start_line": 1, "end_line": 1}
        self.apk_hash = file_digest(self.apk)
        self.source_hash = file_digest(self.source)

    def write_archive(self, paths):
        contents = b"class Fixture {}\n"
        with tarfile.open(self.source, "w:gz") as archive:
            for path in paths:
                entry = tarfile.TarInfo(f"{SOURCE_ROOT}/{path}")
                entry.size = len(contents)
                archive.addfile(entry, BytesIO(contents))

    def row(self, key, checks=None, structure=None, source=None):
        at, member = key
        return {"at": at, "member": member, "source": source or self.source_pin,
                "checks": checks or [], "source_structure": structure}

    def report(self, rows, *, source_hash=None, evaluator="evaluator"):
        return {
            "schema_version": 1, "kind": "diana-decbench-run", "backend": "diana", "revision": "rev",
            "binary": {"sha256": "tool"}, "evaluator_sha256": evaluator,
            "inputs": [{"suite": "public:fixture", "sha256": self.apk_hash, "methods": rows,
                        "corpus": {"id": "fixture", "source_archive_sha256": source_hash or self.source_hash}}],
        }

    def test_counts_physical_populations_separately_from_source_files(self):
        report = corpus_coverage.measure(self.apk, self.source, SOURCE_ROOT)
        original = report["original"]
        self.assertEqual(original["method_definitions"], len(self.methods))
        self.assertEqual(original["root_dex_entries"], [{"entry": 1, "name": "classes.dex"},
                                                        {"entry": 2, "name": "classes2.dex"}])
        source = report["source"]
        self.assertEqual(source["files_by_language"], {"java": 2, "kotlin": 2})
        self.assertEqual(source["files_by_source_set"]["main"], {"java": 1, "kotlin": 1})
        self.assertEqual(source["files_by_source_set"]["test"], {"java": 1, "kotlin": 0})
        self.assertEqual(source["files_by_module"], {"app": {"java": 2, "kotlin": 1}})
        self.assertEqual(source["ambiguous_language_paths"], 1)
        self.assertEqual(source["invalid_paths"], 1)
        self.assertIsNone(report["selection"]["original_methods_without_a_matched_case"])

    def test_validates_case_pins_and_keeps_shard_candidate_denominators(self):
        first = next(key for key in self.methods if "->carrier(" in key[1])
        second = next(key for key in self.methods if key[1] == first[1] and key[0] != first[0])
        missing = ("e6:l0:c0:m123", first[1])

        def case(rows, shard):
            return {"schema_version": 1, "source_revision": "0" * 40, "input_sha256": self.apk_hash,
                    "methods": [dict(row, obligations=[{"id": "field", "kind": "field_access", "description": "Field"}])
                                for row in rows],
                    "discovery": {"kind": "example", "candidate_methods": 3,
                                  "limit": 10, "shard": {"count": 2, "number": shard}}}

        bad_pin = dict(self.source_pin, end_line=2)
        result = corpus_coverage.measure(self.apk, self.source, SOURCE_ROOT, cases=[
            case([self.row(first), self.row(missing)], 1), case([self.row(first), self.row(second, source=bad_pin)], 2),
        ])["selection"]
        self.assertEqual(result["matched_original_methods"], 2)
        self.assertEqual(result["matched_original_and_source_pin_methods"], 1)
        self.assertEqual(result["matched_source_pin_methods_by_language"], {"java": 1})
        self.assertEqual(result["matched_source_pin_methods_by_obligation_kind"], {"field_access": 1})
        self.assertEqual(result["original_methods_without_a_matched_case"], len(self.methods) - 2)
        self.assertEqual([manifest["discovery"]["candidate_methods"] for manifest in result["manifests"]], [3, 3])
        self.assertEqual(result["manifests"][0]["unmatched_original_methods"], 1)
        self.assertEqual(result["manifests"][1]["unmatched_source_pins_for_matched_originals"], 1)

    def test_counts_actual_receipts_and_separates_incompatible_provenance(self):
        first, second, third, *remaining = self.methods
        checked = self.row(first, [{"metric": "corpus_quality", "kind": "call_presence", "status": "PASS"},
                                   {"metric": "corpus_quality", "kind": "call_sequence", "status": "UNCHECKED"}],
                           {"outcome": "DIFFERENT"})
        unavailable = self.row(second, [{"metric": "corpus_quality", "status": "SETUP_ERROR"}],
                               {"outcome": "UNCHECKED"})
        identity_only = self.row(third, [{"metric": "identity", "status": "PASS"},
                                         {"metric": "body_recovery", "status": "PASS"}])
        wrong_member = self.row((third[0], first[1]), [{"metric": "corpus_quality", "status": "PASS"}])
        wrong_source = self.row(remaining[0], [{"metric": "corpus_quality", "status": "PASS"}],
                                source=dict(self.source_pin, sha256="0" * 64))
        report = corpus_coverage.measure(self.apk, self.source, SOURCE_ROOT, reports=[
            self.report([checked, unavailable, identity_only, wrong_member, wrong_source]), self.report([checked]),
            self.report([checked], source_hash="different"), self.report([checked], evaluator="new"),
        ])["evidence"]
        first_snapshot = report["snapshots"][0]["inputs"][0]
        self.assertEqual(first_snapshot["unmatched_original_rows"], 1)
        self.assertEqual(first_snapshot["unmatched_source_pins_for_matched_originals"], 1)
        self.assertIn("unavailable", report["snapshots"][2]["inputs"][0])
        combined, distinct = report["unique_methods_by_tool_and_evaluator"]
        self.assertEqual(combined["reports"], [1, 2])
        self.assertEqual(distinct["reports"], [4])
        quality = combined["families"]["corpus_quality"]
        self.assertEqual(quality["methods_with_receipts"], 2)
        self.assertEqual(quality["methods_with_any_measured_receipt"], 1)
        self.assertEqual(quality["methods_with_only_measured_receipts"], 0)
        self.assertEqual(quality["methods_by_receipt_state"], {"PASS": 1, "FAIL": 0, "UNCHECKED": 1, "SETUP_ERROR": 1})
        self.assertEqual(combined["methods_with_measured_quality_checks"], 1)
        self.assertEqual(combined["source_structure"]["methods_with_any_measured_receipt"], 1)
        self.assertEqual(combined["obligation_kinds"]["call_presence"]["methods_with_any_measured_receipt"], 1)

    def test_independent_denominator_remains_available_when_source_root_is_absent(self):
        report = corpus_coverage.measure(self.apk, self.source, "different-root")
        self.assertFalse(report["source"]["complete"])
        self.assertIn("source root is absent", report["source"]["unavailable"])
        self.assertEqual(report["original"]["method_definitions"], len(self.methods))


if __name__ == "__main__":
    unittest.main()
