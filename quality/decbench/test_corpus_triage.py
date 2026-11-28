"""Verify fresh single-method triage against independently specified transitions."""

from copy import deepcopy
import json
from pathlib import Path
from tempfile import TemporaryDirectory
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import corpus_triage


AT = "e0:l0:c0:m7"
MEMBER = "Ldemo/Subject;->apply()V"
PINS = {"input_sha256": "a" * 64, "oracle_sha256": "b" * 64,
        "source_archive_sha256": "c" * 64, "member": MEMBER}
SOURCE = {"url": "https://example.com/pinned/Subject.java#L2-L6"}


def check(identifier, status, expected=1):
    return {"id": identifier, "metric": "modeled_behavior", "status": status, "expected": expected,
            "observed": expected if status == "PASS" else None}


def method(*checks, at=AT, member=MEMBER, text="base"):
    return {"at": at, "member": member, "source": SOURCE, "checks": list(checks),
            "status": "PASS", "observation": {"text": text}}


class CorpusTriageTest(unittest.TestCase):
    def setUp(self):
        self.root = Path(self.enterContext(TemporaryDirectory()))
        self.saved = self.root / "saved.json"
        self.binary = self.root / "current-diana"
        self.cache = self.root / "prepared"
        self.jar = self.root / "jadx.jar"
        self.project = SimpleNamespace(id="sample", source_commit="d" * 40,
                                       manifest=self.root / "catalog" / "sample.json")
        self.lookup = self.enterContext(patch.object(corpus_triage.corpus_assets, "project_by_id",
                                                   return_value=self.project))
        self.run = self.enterContext(patch.object(corpus_triage.corpus_eval, "run"))

    def report(self, *methods, backend="diana", external=False, profile="modeled", structure=False):
        return {"kind": "diana-decbench-run", "schema_version": 1, "backend": backend,
                "binary": {"path": f"/report/ignored/{backend}", "sha256": "e" * 64},
                "prepared": str(self.cache), "inventory_complete": True, "inputs": [{
                    "suite": "public:sample", "sha256": PINS["input_sha256"],
                    "oracle_sha256": PINS["oracle_sha256"], "inventory": {"complete": True},
                    "corpus": {"id": "sample", "source_revision": self.project.source_commit,
                               "source_archive_sha256": PINS["source_archive_sha256"],
                               "execute_models": profile == "modeled", "inspect_structure": structure,
                               "profile": profile, "external_cases": external,
                               "cases": str(self.root / "ignored.json")}, "methods": list(methods),
                }]}

    def write(self, document):
        self.saved.write_text(json.dumps(document), encoding="utf-8")

    def triage(self, **options):
        return corpus_triage.triage(self.saved, AT, self.binary, **options)

    def test_fresh_exact_method_produces_separate_failure_categories_and_peer_observation(self):
        self.write(self.report(method(check("reproduced", "FAIL"), check("resolved", "FAIL"),
                                      check("new", "PASS"), check("measured", "UNCHECKED")),
                               method(check("irrelevant", "FAIL"), at="e5:l0:c0:m7")))
        self.run.side_effect = [
            self.report(method(check("reproduced", "FAIL"), check("resolved", "PASS"), check("new", "FAIL"),
                               check("measured", "FAIL"), text="fresh Diana")),
            self.report(method(check("new", "PASS"), text="fresh peer"), backend="jadx"),
        ]
        receipt = self.triage(jadx_jar=self.jar, revision="current-branch")
        self.assertEqual(receipt["summary"], {"NEW_FAILURE": 1, "REPRODUCED": 1, "RESOLVED": 1,
                                              "OBSERVED_FAILURE": 1})
        self.assertEqual(receipt["status"], "NEW_FAILURE")
        self.assertEqual(receipt["baseline"]["observation"]["text"], "base")
        self.assertEqual(receipt["current"]["observation"]["text"], "fresh Diana")
        self.assertEqual(receipt["peer"]["observation"]["text"], "fresh peer")
        self.assertEqual(receipt["source"]["url"], SOURCE["url"])
        self.assertEqual(corpus_triage.result_code(receipt), 1)
        for expected, producer, revision, backend in zip(self.run.call_args_list, (self.binary, self.jar),
                                                         ("current-branch", None), ("diana", "jadx")):
            self.assertEqual(expected.args, (self.project, self.cache, producer, revision, backend, [AT], 1, 60))
            self.assertEqual(expected.kwargs, {"expected_evidence": PINS, "case_path": None,
                                               "execute_models": True, "inspect_structure": False})

    def test_uncertain_checks_do_not_hide_independently_reproduced_failure(self):
        original = method(check("known", "FAIL"), check("unavailable", "FAIL"), check("oracle", "FAIL", True))
        current = method(check("known", "FAIL"), check("unavailable", "UNCHECKED"), check("oracle", "PASS", 1))
        self.write(self.report(original))
        self.run.return_value = self.report(current)
        receipt = self.triage()
        self.assertEqual(receipt["status"], "EVIDENCE_CHANGED")
        self.assertEqual(receipt["summary"], {"REPRODUCED": 1, "LOCAL_UNCERTAINTY": 1, "EVIDENCE_CHANGED": 1})
        self.assertEqual(corpus_triage.result_code(receipt), 2)

    def test_unchecked_review_shape_keeps_the_semantic_counterexample(self):
        original = method(check("known", "FAIL"))
        current = method(check("known", "FAIL"))
        current["source_structure"] = {"outcome": "UNCHECKED", "detail": "unsupported protected statements"}
        self.write(self.report(original, structure=True))
        self.run.return_value = self.report(current, structure=True)
        receipt = self.triage()
        self.assertEqual(receipt["status"], "REPRODUCED")
        self.assertEqual(receipt["current"]["source_structure"]["outcome"], "UNCHECKED")
        self.assertEqual(corpus_triage.result_code(receipt), 1)

    def test_changed_pin_stops_before_any_peer_attempt_and_preserves_saved_evidence(self):
        self.write(self.report(method(check("failed", "FAIL"))))
        self.run.side_effect = corpus_triage.corpus_eval.EvidenceChanged("saved case evidence differs in oracle_sha256")
        receipt = self.triage(jadx_jar=self.jar)
        self.run.assert_called_once()
        self.assertEqual(receipt["status"], "EVIDENCE_CHANGED")
        self.assertEqual(receipt["pins"], PINS)
        self.assertIn("oracle_sha256", receipt["detail"])
        self.assertNotIn("current", receipt)
        self.assertNotIn("peer", receipt)
        self.assertEqual(corpus_triage.result_code(receipt), 2)

    def test_changed_evaluator_stops_before_either_producer(self):
        saved = self.report(method(check("failed", "FAIL")))
        self.write(dict(saved, evaluator_sha256="previous"))
        receipt = self.triage(jadx_jar=self.jar)
        self.run.assert_not_called()
        self.assertEqual(receipt["status"], "EVIDENCE_CHANGED")
        self.assertIn("evaluator", receipt["detail"])
        self.assertEqual(corpus_triage.result_code(receipt), 2)

    def test_does_not_trust_a_fresh_report_with_different_identity_or_asset(self):
        self.write(self.report(method(check("failed", "FAIL"))))
        for field, value in (("sha256", "f" * 64), ("oracle_sha256", "f" * 64)):
            with self.subTest(field=field):
                self.run.return_value = self.report(method(check("failed", "PASS")))
                self.run.return_value["inputs"][0][field] = value
                receipt = self.triage()
                self.assertEqual(receipt["status"], "EVIDENCE_CHANGED")
                self.assertEqual(receipt["checks"], [])
        self.run.return_value = self.report(method(check("failed", "PASS"), member="Ldemo/Subject;->other()V"))
        self.assertEqual(self.triage()["status"], "EVIDENCE_CHANGED")

    def test_uses_only_a_contained_snapshot_or_explicit_external_case_file(self):
        original = self.report(method(check("selected", "PASS")), external=True, profile="discovered", structure=True)
        self.write(original)
        outside = Path(self.enterContext(TemporaryDirectory())) / "cases.json"
        outside.write_text("{}", encoding="utf-8")
        # The saved absolute manifest is not used automatically.
        original["inputs"][0]["corpus"]["cases"] = str(outside)
        self.write(original)
        self.assertIn("--cases", self.triage()["detail"])
        self.run.assert_not_called()
        snapshot = self.root / "cases.json"
        snapshot.symlink_to(outside)
        self.assertIn("escapes", self.triage()["detail"])
        self.run.assert_not_called()
        self.run.return_value = deepcopy(original)
        # Use the explicit manifest when replaying a standalone saved report.
        self.assertEqual(self.triage(case_path=outside, cache=self.root / "new-cache")["status"], "PASS")
        self.assertEqual(self.run.call_args.kwargs["case_path"], outside)
        self.assertEqual(self.run.call_args.args[1], self.root / "new-cache")
        snapshot.unlink()
        snapshot.write_text("{}", encoding="utf-8")
        self.assertEqual(self.triage()["status"], "PASS")
        self.assertEqual(self.run.call_args.kwargs["case_path"], snapshot)
        self.assertEqual(self.run.call_args.kwargs["inspect_structure"], True)

    def test_peer_setup_error_is_scoped_to_peer_and_does_not_erase_resolution(self):
        self.write(self.report(method(check("fixed", "FAIL"))))
        self.run.side_effect = [self.report(method(check("fixed", "PASS"))), ValueError("JADX is missing")]
        receipt = self.triage(jadx_jar=self.jar)
        self.assertEqual(receipt["status"], "RESOLVED")
        self.assertEqual(receipt["peer"], {"capture_status": "SETUP_ERROR", "detail": "JADX is missing"})
        self.assertEqual(corpus_triage.result_code(receipt), 2)

    def test_wrong_backend_or_repeated_physical_selection_never_launches_producer(self):
        self.write(self.report(method(), backend="jadx"))
        self.assertEqual(self.triage()["status"], "SETUP_ERROR")
        self.write(self.report(method(), method(member="Ldemo/Subject;->other()V")))
        self.assertEqual(self.triage()["status"], "SETUP_ERROR")
        self.run.assert_not_called()


if __name__ == "__main__":
    unittest.main()
