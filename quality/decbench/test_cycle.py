"""Verify that automatic repair acceptance uses the whole measured scope."""

from copy import deepcopy
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
        other["checks"][0].update(status="FAIL", observed=0)
        self.assertFalse(cycle.gate(baseline, passing, case)[1])
        other["checks"][0].update(status="UNCHECKED", observed=None)
        self.assertFalse(cycle.gate(baseline, passing, case)[1])
        other["checks"][0].update(status="PASS", observed=7)
        passing["inputs"][0]["oracle_sha256"] = "changed"
        self.assertFalse(cycle.gate(baseline, passing, case)[1])
