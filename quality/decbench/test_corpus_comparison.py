"""Check campaign pairing, evidence transitions, and bundle containment."""

import json
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import patch

import corpus_comparison


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value), encoding="utf-8")


def check(identifier, status, expected=1):
    return {"id": identifier, "metric": "corpus_quality", "expected": expected,
            "observed": expected if status == "PASS" else None, "status": status}


def method(number=1, checks=None, signature=None):
    return {"at": f"e0:l0:c0:m{number}", "member": signature or f"Lfixture/Sample;->body{number}()V",
            "checks": [check("selected", "PASS")] if checks is None else checks,
            "observation": {"text": "<method evidence>"}, "status": "PASS"}


class CampaignComparisonTest(unittest.TestCase):
    def setUp(self):
        self.root = Path(self.enterContext(TemporaryDirectory()))

    def receipt(self, label, *, shards=1, jobs=4, profile="reviewed"):
        data = {"kind": "diana-decbench-campaign", "schema_version": 1, "shards": shards,
                "jobs": jobs, "profile": profile, "execute_models": False, "inspect_structure": False,
                "producer_sha256": {"diana": "a" * 64, "jadx": "b" * 64}, "units": []}
        path = self.root / label / "campaign.json"
        write(path, data)
        return path, data

    def add(self, bundle, methods, *, corpus="sample", backend="diana", shard=1,
            apk="c" * 64, oracle="d" * 64, inventory=None):
        path, receipt = bundle
        inventory = inventory or {"complete": True}
        checks = {item["status"] for method in methods for item in method["checks"]}
        incomplete = not inventory["complete"] or bool(checks & {"UNCHECKED", "SETUP_ERROR"})
        code = 2 if incomplete else int("FAIL" in checks)
        reference = f"{corpus}/shard-{shard:04d}/{backend}.json"
        report = {"kind": "diana-decbench-run", "schema_version": 1, "backend": backend,
                  "binary": {"path": "/external/producer", "sha256": receipt["producer_sha256"][backend]},
                  "inventory_complete": inventory["complete"], "inputs": [{
                      "suite": f"public:{corpus}", "sha256": apk, "oracle_sha256": oracle,
                      "path": "/external/input.apk", "inventory": inventory, "methods": methods,
                  }]}
        unit = {"corpus": corpus, "backend": backend, "stage": "evaluation", "shard": shard,
                "shards": receipt["shards"], "selected": [item["at"] for item in methods], "report": reference,
                "exit_code": code, "status": ("PASS", "FAIL", "INCOMPLETE")[code]}
        receipt["units"].append(unit)
        write(path.parent / reference, report)
        write(path, receipt)
        return unit

    def test_same_executable_and_different_jobs_compare_check_transitions(self):
        before = self.receipt("before", jobs=1)
        after = self.receipt("after", jobs=8)
        self.add(before, [method(checks=[check("regressed", "PASS"), check("fixed", "FAIL"),
                                        check("lost", "PASS"), check("different", "PASS", True)])])
        self.add(after, [method(checks=[check("regressed", "FAIL"), check("fixed", "PASS"),
                                       check("different", "PASS", 1)])])
        result = corpus_comparison.compare(before[0].parent, after[0])
        self.assertEqual(result["scope"], {
            "compared": 1, "partial": 0, "coverage_changed": 0, "incomplete": 0, "unpaired": 0,
        })
        self.assertEqual(result["summary"], {"newly_failing": 1, "recovered": 1, "coverage_added": 0,
                                             "coverage_lost": 1, "coverage_changed": 1, "unchanged": 0})
        self.assertEqual(result["baseline"]["jobs"], 1)
        self.assertEqual(result["candidate"]["jobs"], 8)
        self.assertEqual(corpus_comparison.result_code(result), 1)
        self.assertIn("&lt;method evidence&gt;", corpus_comparison.render(result))

    def test_partial_checks_compare_without_claiming_complete_coverage(self):
        before, after = self.receipt("before"), self.receipt("after")
        self.add(before, [method(checks=[check("availability", "PASS"), check("observable", "PASS")])])
        self.add(after, [method(checks=[check("availability", "UNCHECKED"), check("observable", "FAIL")])])
        after[1]["units"][0].update(status="PASS", exit_code=0)
        write(*after)
        result = corpus_comparison.compare(before[0], after[0])
        self.assertEqual(result["scope"]["partial"], 1)
        self.assertEqual(result["summary"]["newly_failing"], 1)
        self.assertEqual(result["summary"]["coverage_lost"], 1)
        self.assertEqual(corpus_comparison.result_code(result), 1)

    def test_same_unchecked_scope_passes_guard_and_a_setup_error_blocks_it(self):
        before, after = self.receipt("before"), self.receipt("after")
        checks = [check("known-uncertainty", "UNCHECKED"), check("observable", "PASS")]
        self.add(before, [method(checks=checks)])
        unit = self.add(after, [method(checks=checks)])
        result = corpus_comparison.compare(before[0], after[0])
        self.assertEqual(result["scope"]["partial"], 1)
        self.assertEqual(result["summary"]["unchanged"], 2)
        self.assertEqual(corpus_comparison.result_code(result), 0)
        path = after[0].parent / unit["report"]
        report = json.loads(path.read_text())
        report["inputs"][0]["methods"][0]["checks"][0]["status"] = "SETUP_ERROR"
        write(path, report)
        result = corpus_comparison.compare(before[0], after[0])
        self.assertEqual(result["scope"]["incomplete"], 1)
        self.assertEqual(corpus_comparison.result_code(result), 2)

    def test_changed_pins_members_or_locator_selection_are_scope_changes(self):
        for label, kwargs, methods in (
            ("different-apk", {"apk": "e" * 64}, [method()]),
            ("different-oracle", {"oracle": "e" * 64}, [method()]),
            ("different-member", {}, [method(signature="Lfixture/Sample;->other()V")]),
            ("different-locator", {}, [method(number=2)]),
        ):
            with self.subTest(label=label):
                before, after = self.receipt(f"before-{label}"), self.receipt(label)
                self.add(before, [method()])
                self.add(after, methods, **kwargs)
                result = corpus_comparison.compare(before[0], after[0])
                self.assertEqual(result["scope"]["coverage_changed"], 1)
                self.assertEqual(sum(result["summary"].values()), 0)
                self.assertEqual(corpus_comparison.result_code(result), 2)

    def test_unpaired_units_and_setup_diagnostics_do_not_erase_matched_scope(self):
        before, after = self.receipt("before"), self.receipt("after")
        self.add(before, [method()], corpus="matched")
        self.add(after, [method()], corpus="matched")
        self.add(before, [method(checks=[check("fixed", "FAIL")])], corpus="interrupted")
        self.add(after, [method()], corpus="interrupted", inventory={"complete": False, "error": "worker timeout"})
        self.add(before, [method()], backend="jadx", corpus="baseline-extra")
        after[1]["units"].append({"corpus": "offline", "stage": "prepare", "status": "SETUP_ERROR",
                                  "exit_code": 2, "detail": "APK absent from cache"})
        write(*after)
        result = corpus_comparison.compare(before[0], after[0])
        self.assertEqual(result["scope"], {"compared": 1, "partial": 0, "coverage_changed": 0,
                                           "incomplete": 2, "unpaired": 1})
        self.assertEqual(result["summary"]["recovered"], 0)
        self.assertEqual(result["summary"]["unchanged"], 1)
        self.assertEqual(result["preparation"][0]["detail"], "APK absent from cache")
        interrupted = next(unit for unit in result["units"] if unit["corpus"] == "interrupted")
        self.assertEqual(interrupted["detail"], "worker timeout")

    def test_receipt_references_cannot_read_reports_outside_each_bundle(self):
        before, after = self.receipt("before"), self.receipt("after")
        self.add(before, [method()])
        unit = self.add(after, [method()])
        outside = self.root / "outside.json"
        outside.write_bytes((after[0].parent / unit["report"]).read_bytes())
        link = after[0].parent / "outside-link.json"
        link.symlink_to(outside)
        for reference in ("../outside.json", str(outside), link.name):
            with self.subTest(reference=reference):
                unit["report"] = reference
                write(*after)
                with patch("corpus_comparison.read_document", wraps=corpus_comparison.read_document) as read:
                    result = corpus_comparison.compare(before[0], after[0])
                self.assertFalse(any(call.args[0] == outside for call in read.call_args_list))
                self.assertEqual(result["scope"]["incomplete"], 1)
                self.assertEqual(sum(result["summary"].values()), 0)

    def test_same_method_cannot_be_counted_twice_across_shards(self):
        before, after = self.receipt("before", shards=2), self.receipt("after", shards=2)
        self.add(before, [method()], shard=1)
        self.add(before, [method()], shard=2)
        self.add(after, [method()], shard=1)
        with self.assertRaisesRegex(ValueError, "repeats a unit or assigned method"):
            corpus_comparison.compare(before[0], after[0])


if __name__ == "__main__":
    unittest.main()
