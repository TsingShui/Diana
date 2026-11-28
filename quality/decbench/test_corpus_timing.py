"""Exercise paired latency evidence and exclusions without relying on host timings."""

from pathlib import Path
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import patch

import corpus_timing as timing

OWNER = "Lfixture/Example;"
FIRST = {"at": "e0:l0:c0:m1", "member": OWNER + "->first()I"}
SECOND = {"at": "e0:l0:c0:m2", "member": OWNER + "->second()I"}


class TimingTest(unittest.TestCase):
    def setUp(self):
        scratch = TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        root = Path(scratch.name)
        self.before, self.after, self.apk, self.source = (
            root / name for name in ("baseline", "candidate", "fixture.apk", "source.tar.gz")
        )
        for path in (self.before, self.after, self.apk, self.source):
            path.write_text(path.name, encoding="utf-8")
        self.calls = []
        self.selections = [FIRST, SECOND]
        self.inventory_members = {(row["at"], row["member"]) for row in self.selections}

    def inventory(self, binary, path, timeout):
        self.calls.append(("inventory", binary.name))
        self.assertEqual(path, self.apk)
        self.assertGreater(timeout, 0)
        elapsed = 1.0 if binary == self.before else 2.0
        return self.inventory_members.copy(), {"complete": True, "exit_code": 0, "elapsed_seconds": elapsed}

    def retrieve(self, argv, timeout):
        binary, _, artifact, at, *flags = argv
        self.calls.append((at, Path(binary).name))
        self.assertEqual(artifact, str(self.apk))
        self.assertEqual(flags, ["--format", "json"])
        self.assertGreater(timeout, 0)
        elapsed = 0.25 if binary == str(self.before) else 0.5
        return {
            "exit_code": 0, "elapsed_seconds": elapsed,
            "publication": {"schema_version": 5, "command": "get-method",
                            "selection": {"method_at": f"{at}::{OWNER}->"},
                            "result": {"text": "int first() { return 1; }"}, "issues": []},
        }

    def measure(self, inventory=None, retrieve=None):
        with (patch("corpus_timing.evidence.inventory", side_effect=inventory or self.inventory),
              patch("corpus_timing.evidence.run_cli", side_effect=retrieve or self.retrieve)):
            return timing.measure(self.before, self.after, self.apk, self.selections, repeats=2,
                                  source_archive=self.source)

    def test_pairs_alternate_and_warmup_is_excluded_from_medians(self):
        report = self.measure()
        self.assertTrue(report["comparable"])
        self.assertEqual(report["warmup"]["order"], ["baseline", "candidate"])
        operations = ["inventory", FIRST["at"], SECOND["at"]]
        expected = []
        for order in (("baseline", "candidate"), ("candidate", "baseline"), ("baseline", "candidate")):
            expected.extend((operation, arm) for operation in operations for arm in order)
        self.assertEqual(self.calls, expected)
        summary = report["comparison"]
        self.assertEqual(summary["inventory"]["baseline"], {"median_seconds": 1.0, "total_seconds": 2.0})
        self.assertEqual(summary["method_totals"]["candidate"], {"median_seconds": 1.0, "total_seconds": 2.0})
        self.assertEqual(summary["methods"][0]["comparison"]["median_paired_delta_seconds"], 0.25)
        self.assertEqual(summary["methods"][0]["comparison"]["paired_deltas"],
                         {"median_absolute_deviation_seconds": 0.0, "minimum_seconds": 0.25,
                          "maximum_seconds": 0.25, "candidate_faster": 0, "candidate_slower": 2, "tied": 0})
        self.assertEqual(summary["methods"][0]["publication"],
                         {"distinct_outputs": {"baseline": 1, "candidate": 1}, "identical_all_trials": True})
        self.assertEqual(report["measurements"][0]["methods"][0]["candidate"]["publication_bytes"], 25)
        self.assertEqual(report["provenance"]["source_archive"]["sha256"], timing.evidence.file_digest(self.source))
        self.assertFalse(report["provenance"]["same_binary_bytes"])

    def test_incomplete_publication_defeats_its_method_and_aggregate(self):
        def incomplete(argv, timeout):
            row = self.retrieve(argv, timeout)
            if argv[0] == str(self.after) and argv[3] == FIRST["at"]:
                row["exit_code"] = 1
                row["publication"]["issues"] = [{"code": "missing-body"}]
            return row

        report = self.measure(retrieve=incomplete)
        self.assertFalse(report["comparable"])
        self.assertIsNone(report["comparison"]["method_totals"])
        self.assertIsNone(report["comparison"]["methods"][0]["comparison"])
        self.assertIsNotNone(report["comparison"]["methods"][1]["comparison"])
        row = report["measurements"][0]["methods"][0]["candidate"]
        self.assertEqual(row["elapsed_seconds"], 0.5)
        self.assertEqual(row["issues"], [{"code": "missing-body"}])

    def test_inventory_members_must_match_selected_descriptors(self):
        def wrong_member(binary, path, timeout):
            members, observation = self.inventory(binary, path, timeout)
            if binary == self.after:
                members.remove((FIRST["at"], FIRST["member"]))
                members.add((FIRST["at"], OWNER + "->first()J"))
            return members, observation

        report = self.measure(inventory=wrong_member)
        self.assertFalse(report["comparable"])
        self.assertTrue(report["comparison"]["inventory_mismatch"])
        self.assertIsNone(report["comparison"]["inventory"])
        self.assertIsNone(report["comparison"]["method_totals"])
        self.assertNotIn((FIRST["at"], "candidate"), self.calls)
        self.assertEqual(report["warmup"]["inventory"]["candidate"]["mismatches"][0]["observed"], OWNER + "->first()J")
        self.assertIsNotNone(report["comparison"]["methods"][1]["comparison"])

    def test_contradictory_publication_locator_is_not_paired(self):
        def wrong_locator(argv, timeout):
            result = self.retrieve(argv, timeout)
            if argv[0] == str(self.after):
                result["publication"]["selection"]["method_at"] += "wrong()V"
            return result

        report = self.measure(retrieve=wrong_locator)
        self.assertFalse(report["comparable"])
        self.assertIsNotNone(report["comparison"]["inventory"])
        self.assertIsNone(report["comparison"]["method_totals"])
        self.assertTrue(all(row["comparison"] is None for row in report["comparison"]["methods"]))

    def test_changing_verified_input_invalidates_all_pair_summaries(self):
        def changed_input(argv, timeout):
            result = self.retrieve(argv, timeout)
            self.apk.write_bytes(b"new content during the run")
            return result

        report = self.measure(retrieve=changed_input)
        self.assertFalse(report["comparable"])
        self.assertEqual(report["errors"], ["artifact changed during the benchmark"])
        self.assertIsNone(report["comparison"]["inventory"])
        self.assertIsNone(report["comparison"]["method_totals"])

    def test_full_time_budget_preserves_unmeasured_rows(self):
        with (patch("corpus_timing.evidence.inventory", side_effect=self.inventory),
              patch("corpus_timing.evidence.run_cli", side_effect=self.retrieve)):
            report = timing.measure(self.before, self.after, self.apk, self.selections,
                                    time_budget=1e-12)
        self.assertFalse(report["comparable"])
        self.assertEqual(report["measurements"], [])
        self.assertEqual(self.calls, [])
        self.assertIn("time budget", report["warmup"]["inventory"]["baseline"]["error"])
