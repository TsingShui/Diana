"""Verify that automatic repair acceptance uses the whole measured scope."""

from copy import deepcopy
from json import loads
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest

import candidates
import cycle


def fixture_report() -> dict:
    return {
        "kind": "diana-decbench-run", "schema_version": 1,
        "inventory_complete": True, "checks_by_status": {"FAIL": 1, "PASS": 1},
        "inputs": [{
            "suite": "fixture", "sha256": "input", "oracle_sha256": "oracle",
            "methods": [
                {"at": f"j0:m{index}", "member": f"LFoo;->m{index}()I", "status": status,
                 "checks": [{"id": "return", "metric": "behavior", "status": status,
                             "expected": 7, "observed": 7 if status == "PASS" else 8}]}
                for index, status in enumerate(("FAIL", "PASS"))
            ],
        }],
    }


class CycleTest(unittest.TestCase):
    def test_requires_the_leased_failure_to_recover_without_losing_evidence(self):
        baseline = fixture_report()
        artifact = baseline["inputs"][0]
        case = candidates.identity(artifact, artifact["methods"][0])
        self.assertFalse(cycle.gate(baseline, baseline, case)[1])
        passing = deepcopy(baseline)
        passing["checks_by_status"] = {"PASS": 2}
        repaired, other = passing["inputs"][0]["methods"]
        repaired["status"] = "PASS"
        repaired["checks"][0].update(status="PASS", observed=7)
        comparison, accepted = cycle.gate(baseline, passing, case)
        self.assertTrue(accepted)
        self.assertEqual(comparison["summary"]["recovered"], 1)
        self.assertFalse(cycle.gate(baseline, dict(passing, evaluator_sha256="different"), case)[1])
        other["checks"][0].update(status="FAIL", observed=0)
        self.assertFalse(cycle.gate(baseline, passing, case)[1])
        other["checks"][0].update(status="UNCHECKED", observed=None)
        self.assertFalse(cycle.gate(baseline, passing, case)[1])
        other["checks"][0].update(status="PASS", observed=7)
        passing["inputs"][0]["oracle_sha256"] = "changed"
        self.assertFalse(cycle.gate(baseline, passing, case)[1])

    def test_public_cycle_preserves_known_unchecked_paths_outside_the_candidate(self):
        baseline = fixture_report()
        artifact = baseline["inputs"][0]
        case = candidates.identity(artifact, artifact["methods"][0])
        baseline["checks_by_status"] = {"FAIL": 1, "UNCHECKED": 1}
        other = artifact["methods"][1]
        other["status"] = "UNCHECKED"
        other["checks"][0].update(status="UNCHECKED", observed=None)
        candidate = deepcopy(baseline)
        candidate["checks_by_status"] = {"PASS": 1, "UNCHECKED": 1}
        repaired = candidate["inputs"][0]["methods"][0]
        repaired["status"] = "PASS"
        repaired["checks"][0].update(status="PASS", observed=7)
        self.assertTrue(cycle.gate(baseline, candidate, case, public=True)[1])
        self.assertFalse(cycle.gate(baseline, candidate, case)[1])
        other_after = candidate["inputs"][0]["methods"][1]
        other_after["checks"][0].update(status="SETUP_ERROR")
        candidate["checks_by_status"] = {"PASS": 1, "SETUP_ERROR": 1}
        self.assertFalse(cycle.gate(baseline, candidate, case, public=True)[1])

    def test_guard_requires_the_same_original_diana_executable(self):
        original = dict(fixture_report(), binary={"sha256": "original"})
        other = dict(fixture_report(), binary={"sha256": "different"})
        with self.assertRaisesRegex(ValueError, "guard.json.*same Diana executable"):
            cycle.guard_plans(original, [(Path("guard.json"), other)], 1, 1)

    def test_guard_failure_keeps_all_review_receipts_and_does_not_require_repairing_existing_failures(self):
        baseline = fixture_report()
        candidate = dict(fixture_report(), binary={"sha256": "candidate"})
        regression = deepcopy(candidate)
        regression["inputs"][0]["methods"][1]["status"] = "FAIL"
        regression["inputs"][0]["methods"][1]["checks"][0].update(status="FAIL", observed=9)
        regression["checks_by_status"] = {"FAIL": 2}

        def unavailable(**_):
            raise OSError("cached source archive is missing")

        guards = [
            (Path("missing.json"), baseline, False, unavailable),
            (Path("known-failures.json"), baseline, False, lambda **_: candidate),
            (Path("new-failure.json"), baseline, False, lambda **_: regression),
        ]
        with TemporaryDirectory() as scratch:
            results = cycle.evaluate_guards(guards, Path(scratch), Path("candidate"), None, "candidate")
            self.assertEqual([result["status"] for result in results], ["incomplete", "passed", "rejected"])
            self.assertIn("cached source archive", results[0]["error"])
            self.assertEqual(loads(Path(results[2]["comparison"]).read_text())["summary"]["newly_failing"], 1)
