"""Check independent verdicts, reproducibility and the repair queue."""

from contextlib import closing
import json
import os
from pathlib import Path
import tempfile
import unittest

import candidates
import judge


AT = "j0:m0"
MEMBER = "Lfixture/Example;->value()I"
SPEC = {"at": AT, "member": MEMBER, "issues": [], "contains": ["return 7;"]}

FAKE = """#!/usr/bin/env python3
import json
from pathlib import Path
import sys

command, source, *rest = sys.argv[1:]
input_text = Path(source).read_text()
if command == "methods":
    print(json.dumps({
        "schema_version": 2, "command": "methods",
        "items": [{"at": "j0:m0", "member": "Lfixture/Example;->value()I"}],
    }))
else:
    assert command == "get-method" and rest[0] == "j0:m0"
    print(json.dumps({
        "schema_version": 5, "command": "get-method",
        "selection": {"method_at": "j0:m0"},
        "result": {"text": "int value() {return " + input_text + ";}"},
        "issues": [],
    }))
"""


class JudgmentTest(unittest.TestCase):
    def test_absence_and_incomplete_inventory_are_distinct(self):
        checks = judge.fact_checks(SPEC, set(), {"complete": True}, None)
        self.assertEqual(checks[0]["status"], "FAIL")
        checks = judge.fact_checks(SPEC, set(), {"complete": False}, None)
        self.assertEqual(checks[0]["status"], "UNCHECKED")
        checks = judge.fact_checks(SPEC, set(), {"complete": False, "error": "timeout"}, None)
        self.assertEqual(checks[0]["status"], "SETUP_ERROR")

    def test_partial_publication_can_satisfy_independent_facts(self):
        seen = {(AT, MEMBER)}
        coverage = {"complete": True}
        issues = [{"context": AT, "kind": "Unsupported", "range_bytes": [4, 7], "reason": "gap"}]
        spec = {**SPEC, "issues": [{"context": AT, "kind": "Unsupported", "range_bytes": [4, 7]}]}
        observation = {"text": "int value() { return 7; }", "issues": issues, "exit_code": 1}
        self.assertTrue(all(item["status"] == "PASS"
                            for item in judge.fact_checks(spec, seen, coverage, observation)))
        observation["issues"].append({"context": AT, "kind": "Malformed"})
        checks = judge.fact_checks(spec, seen, coverage, observation)
        self.assertEqual(judge.status(checks), "FAIL")

    @unittest.skipUnless(os.name == "posix", "the stub CLI needs a shebang")
    def test_one_binary_produces_independent_report_and_checkpoint(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            path = root / "fixture.class"
            path.write_text("8", encoding="utf-8")
            manifest = root / "decbench.json"
            manifest.write_text(json.dumps({
                "schema_version": 1, "id": "example", "input": path.name, "methods": [SPEC],
            }), encoding="utf-8")
            binary = root / "fake"
            binary.write_text(FAKE, encoding="utf-8")
            binary.chmod(0o755)
            arguments = (binary, root, [judge.read_suite(manifest)], None, 1, 5, "diana",
                         root / "checkpoints")
            before = judge.run(*arguments)
            self.assertEqual(before["methods_by_status"], {"FAIL": 1})
            self.assertTrue(judge.run(*arguments)["inputs"][0]["resumed"])
            path.write_text("7", encoding="utf-8")
            after = judge.run(*arguments)
            self.assertEqual(after["methods_by_status"], {"PASS": 1})
            self.assertNotIn("resumed", after["inputs"][0])


class QueueTest(unittest.TestCase):
    def test_leased_case_group_and_recurring_observation(self):
        row = {"at": AT, "member": MEMBER, "status": "FAIL", "checks": [{
            "id": "required", "metric": "behavior", "status": "FAIL", "expected": 7, "observed": 8,
        }]}
        report = {"created_at": "a", "binary": {}, "inputs": [{
            "suite": "example", "path": "/example", "sha256": "input", "oracle_sha256": "oracle",
            "methods": [row],
        }]}
        with tempfile.TemporaryDirectory() as temporary:
            with closing(candidates.connect(Path(temporary))) as db:
                candidates.enqueue(db, report, "before.json")
                candidates.enqueue(db, report, "repeat.json")
                first = candidates.claim(db, 60, None)
                self.assertIsNone(candidates.claim(db, 60, None))
                with self.assertRaises(ValueError):
                    candidates.resolve(db, first["id"], "bad-token", "pending", "bad", None)
                candidates.resolve(db, first["id"], first["claim_token"], "pending",
                                   "First divergent helper effect", "cleanup")
                self.assertIsNone(candidates.claim(db, 60, "unrelated"))
                duplicate = candidates.claim(db, 60, "cleanup")
                self.assertEqual(duplicate["id"], first["id"])

                row["status"] = "PASS"
                report["created_at"] = "b"
                candidates.enqueue(db, report, "pass.json")
                self.assertIsNone(candidates.claim(db, 60, None))
                self.assertEqual(candidates.cases(db, "cleared", "cleanup")[0]["id"], first["id"])

                row["status"] = "FAIL"
                report["created_at"] = "c"
                candidates.enqueue(db, report, "regression.json")
                self.assertEqual(candidates.claim(db, 60, "cleanup")["id"], first["id"])

    def test_new_oracle_supersedes_instead_of_clearing_old_evidence(self):
        row = {"at": AT, "member": MEMBER, "status": "FAIL", "checks": [{
            "id": "expected", "metric": "behavior", "status": "FAIL", "expected": 7, "observed": 8,
        }]}
        artefact = {"suite": "example", "path": "/example", "sha256": "input",
                    "oracle_sha256": "before", "methods": [row]}
        report = {"created_at": "a", "binary": {}, "inputs": [artefact]}
        with tempfile.TemporaryDirectory() as temporary:
            with closing(candidates.connect(Path(temporary))) as db:
                candidates.enqueue(db, report, "first.json")
                old_case = candidates.claim(db, 60, None)
                artefact["oracle_sha256"] = "after"
                report["created_at"] = "b"
                candidates.enqueue(db, report, "second.json")
                self.assertEqual(candidates.cases(db, "superseded", None)[0]["id"], old_case["id"])
                self.assertEqual(len(candidates.cases(db, "pending", None)), 1)
                with self.assertRaises(ValueError):
                    candidates.resolve(db, old_case["id"], old_case["claim_token"],
                                       "ignored", "Old observation", None)

class MetricDispatchTest(unittest.TestCase):
    def test_structural_mismatch_and_unchecked_types_remain_separate(self):
        spec = {"at": AT, "structure": True, "local_types": [{"id": "result"}]}
        checks = []
        judge.append_probe_checks(checks, spec, {
            "structure": ({AT: [{
                "id": "cfg", "status": "FAIL", "expected": {"entry": "switch"},
                "observed": {"entry": "condition"},
            }]}, None),
            "local_types": ({}, None),
        })
        self.assertEqual([(row["metric"], row["status"]) for row in checks],
                         [("structure", "FAIL"), ("local_types", "UNCHECKED")])
        self.assertEqual(checks[0]["id"], "structure:cfg")

    def test_missing_metric_setup_does_not_discard_other_checks(self):
        spec = {"at": AT, "behavior": True, "structure": True}
        checks = []
        judge.append_probe_checks(checks, spec, {
            "behavior": ({AT: [{
                "id": "branch", "status": "PASS", "expected": 7, "observed": 7,
            }]}, None),
            "structure": ({}, "JDK tool unavailable"),
        })
        self.assertEqual([row["status"] for row in checks], ["PASS", "SETUP_ERROR"])
        self.assertEqual(checks[1]["detail"], "JDK tool unavailable")
