"""Check public corpus evaluation using the authored invoke fixture as an APK root."""

from io import BytesIO
import json
from pathlib import Path
from shutil import which
from tarfile import TarInfo, open as open_tar
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import patch
from zipfile import ZipFile

import corpus_assets
import corpus_eval
import evidence


FIXTURE = Path(__file__).resolve().parents[1] / "fixtures" / "atoms" / "invoke-basic"
OWNER = "Lfixture/InvokeBasic;"
SOURCE_PATH = "fixture/InvokeBasic.java"
REVISION = "1" * 40
STATIC = "e0:l0:c2:m8"
INTERFACE = "e0:l0:c2:m5"
MEMBERS = {
    STATIC: f"{OWNER}->staticCall(I)I",
    INTERFACE: f"{OWNER}->interfaceCall(Lfixture/InvokeBasic$Counter;)I",
}
RECOVERED = {
    STATIC: "public static int staticCall(int input) { return Math.abs(input); }",
    INTERFACE: "public static int interfaceCall(Counter input) { return input.next(); }",
}
CAPTURE_METHOD = evidence.method


class CorpusEvaluationTest(unittest.TestCase):
    def setUp(self):
        self.root = Path(self.enterContext(TemporaryDirectory()))
        self.catalog = self.root / "catalog"
        self.catalog.mkdir()
        self.cache = self.root / "assets"
        self.apk = self.root / "local.apk"
        self.source = self.root / "local.tar.gz"
        self.binary = self.root / "producer"
        self.checkpoints = self.root / "checkpoints"
        self.case_path = self.catalog / "invoke-fixture.cases.json"
        self.binary.write_bytes(b"original producer")
        self.write_apk(self.apk)
        self.write_source()
        source_digest = evidence.file_digest(FIXTURE / "InvokeBasic.java")
        self.config = {
            "schema_version": 1, "input_sha256": evidence.file_digest(self.apk),
            "source_revision": REVISION, "revision_entry": "META-INF/version-control-info.textproto",
            "methods": [
                self.case(STATIC, source_digest, 26, 28, "Ljava/lang/Math;", "abs", "(I)I"),
                self.case(INTERFACE, source_digest, 34, 36, f"{OWNER[:-1]}$Counter;", "next", "()I"),
            ],
        }
        self.project = self.save_project(self.config["input_sha256"])
        self.save_cases()
        self.assets = corpus_assets.prepare_assets(self.project, self.cache, apk=self.apk, source=self.source)
        self.inventory = self.enterContext(patch("corpus_eval.evidence.inventory", return_value=(
            set(MEMBERS.items()), {"complete": True},
        )))
        self.capture = self.enterContext(patch("corpus_eval.evidence.method", side_effect=self.observation))

    def case(self, at, digest, start, end, owner, name, descriptor):
        return {
            "at": at, "member": MEMBERS[at],
            "source": {"path": SOURCE_PATH, "sha256": digest, "start_line": start, "end_line": end},
            "obligations": [{
                "id": "retains-call", "kind": "call_presence", "description": "Selected source call",
                "targets": [{"owner": owner, "name": name, "descriptor": descriptor}],
            }],
        }

    def write_apk(self, path, comment=b""):
        with ZipFile(path, "w") as archive:
            archive.write(FIXTURE / "classes.dex", "classes.dex")
            archive.writestr("META-INF/version-control-info.textproto", f'revision: "{REVISION}"\n')
            archive.comment = comment

    def write_source(self, extra=False):
        contents = {SOURCE_PATH: (FIXTURE / "InvokeBasic.java").read_bytes()}
        if extra:
            contents["notes.txt"] = b"Different packaging around the same verified source file."
        with open_tar(self.source, "w:gz") as archive:
            for name, data in contents.items():
                entry = TarInfo(f"release/{name}")
                entry.size = len(data)
                archive.addfile(entry, BytesIO(data))

    def save_project(self, digest):
        (self.catalog / "invoke-fixture.json").write_text(json.dumps({
            "schema_version": 1, "id": "invoke-fixture", "name": "Invoke fixture", "version": "1",
            "apk": {"url": "https://example.org/input.apk", "sha256": digest, "max_bytes": 65536},
            "source": {"url": f"https://example.org/{REVISION}.tar.gz", "max_bytes": 65536},
            "source_commit": REVISION, "source_root": "release",
            "provenance": {"distribution": "Authored fixture"},
        }), encoding="utf-8")
        return corpus_assets.load_projects(self.catalog)["invoke-fixture"]

    def save_cases(self):
        self.case_path.write_text(json.dumps(self.config), encoding="utf-8")

    def observation(self, binary, artifact, at, timeout, *, member):
        self.assertEqual(member, MEMBERS[at])
        return {"text": RECOVERED[at], "issues": [], "exit_code": 0}

    def evaluate(self, selected=None, *, backend="diana", models=False, resume=False, expected_evidence=None):
        return corpus_eval.run(
            self.project, self.cache, self.binary, None, backend, selected, 1, 10,
            execute_models=models, checkpoint=self.checkpoints if resume else None, expected_evidence=expected_evidence,
        )

    def test_invalid_cached_apk_pin_withholds_producer_and_does_not_blame_methods(self):
        self.write_apk(self.assets.apk, comment=b"not the pinned APK")
        report = self.evaluate(resume=True)
        self.inventory.assert_not_called()
        self.capture.assert_not_called()
        self.assertFalse(report["inventory_complete"])
        self.assertEqual(report["methods_by_status"], {"SETUP_ERROR": 2})
        self.assertNotIn("FAIL", report["checks_by_status"])
        self.assertEqual(list(self.checkpoints.glob("*.json")), [])

    def test_fresh_triage_withholds_every_producer_call_when_saved_evidence_changes(self):
        baseline, = self.evaluate([STATIC])["inputs"]
        pins = {"input_sha256": baseline["sha256"], "oracle_sha256": baseline["oracle_sha256"],
                "source_archive_sha256": baseline["corpus"]["source_archive_sha256"], "member": MEMBERS[STATIC]}
        self.assertEqual(self.evaluate([STATIC], expected_evidence=pins)["methods_by_status"], {"PASS": 1})
        self.inventory.reset_mock()
        self.capture.reset_mock()
        with self.assertRaisesRegex(corpus_eval.EvidenceChanged, "member"):
            self.evaluate([STATIC], expected_evidence={**pins, "member": MEMBERS[INTERFACE]})
        self.write_source(extra=True)
        corpus_assets.prepare_assets(self.project, self.cache, source=self.source)
        with self.assertRaisesRegex(corpus_eval.EvidenceChanged, "source_archive_sha256"):
            self.evaluate([STATIC], expected_evidence=pins)
        self.inventory.assert_not_called()
        self.capture.assert_not_called()

    def test_source_setup_failure_does_not_withhold_valid_sibling(self):
        self.config["methods"][1]["source"]["sha256"] = "0" * 64
        self.save_cases()
        report = self.evaluate(resume=True)
        rows = {row["at"]: row for row in report["inputs"][0]["methods"]}
        self.assertEqual(rows[STATIC]["status"], "PASS")
        self.assertEqual(rows[INTERFACE]["status"], "SETUP_ERROR")
        self.assertEqual(report["metrics"]["corpus_quality"], {
            "checks": {"PASS": 1, "SETUP_ERROR": 1},
            "method_rows": {"fully_measured": 1, "unmeasured": 1},
        })
        self.assertEqual(list(self.checkpoints.glob("*.json")), [])

    def test_native_producer_retrieves_only_exact_inventory_identities(self):
        self.inventory.return_value = (
            {(STATIC, MEMBERS[STATIC]), (INTERFACE, f"{OWNER}->interfaceCall(I)I")},
            {"complete": True},
        )
        report = self.evaluate()
        self.capture.assert_called_once_with(self.binary, self.assets.apk, STATIC, 10, member=MEMBERS[STATIC])
        rows = {row["at"]: row for row in report["inputs"][0]["methods"]}
        self.assertEqual(rows[STATIC]["status"], "PASS")
        self.assertIsNone(rows[INTERFACE]["observation"])
        self.assertEqual({check["metric"]: check["status"] for check in rows[INTERFACE]["checks"]}, {
            "identity": "FAIL", "publication": "UNCHECKED", "body_recovery": "UNCHECKED",
            "declaration_types": "UNCHECKED", "corpus_quality": "UNCHECKED",
        })

    def test_native_method_response_cannot_change_its_reported_original_member(self):
        self.capture.side_effect = CAPTURE_METHOD
        for suffix, outcome in ((f"{OWNER}->", "PASS"), (f"{OWNER}->staticCall(J)I", "UNCHECKED")):
            with self.subTest(suffix=suffix):
                answer = {"exit_code": 0, "publication": {
                    "schema_version": 5, "command": "get-method",
                    "selection": {"method_at": f"{STATIC}::{suffix}"},
                    "result": {"text": RECOVERED[STATIC]}, "issues": [],
                }}
                with patch("evidence.run_cli", return_value=answer):
                    report = self.evaluate([STATIC])
                self.assertEqual(report["metrics"]["identity"]["checks"], {"PASS": 1})
                self.assertEqual(report["metrics"]["publication"]["checks"], {outcome: 1})
                self.assertEqual(report["metrics"]["body_recovery"]["checks"], {outcome: 1})
                self.assertEqual(report["metrics"]["corpus_quality"]["checks"], {outcome: 1})

    def test_report_separates_original_code_body_and_neighbor_declaration(self):
        self.capture.side_effect = lambda _, __, at, *args, **kw: {
            "text": (
                "package fixture; public class InvokeBasic { public static int staticCall(int input); "
                "public static int staticCall(long input) { return staticCall((int) input); } }"
                if at == STATIC else RECOVERED[at]
            ),
            "issues": [], "exit_code": 0,
        }
        report = self.evaluate()
        rows = {row["at"]: row for row in report["inputs"][0]["methods"]}
        self.assertGreater(rows[STATIC]["provenance"]["original_code_units"], 0)
        self.assertEqual(report["metrics"]["publication"]["checks"], {"PASS": 2})
        self.assertEqual(report["metrics"]["body_recovery"]["checks"], {"FAIL": 1, "PASS": 1})
        self.assertEqual(report["metrics"]["corpus_quality"]["checks"], {"UNCHECKED": 1, "PASS": 1})

    def test_missing_method_body_cannot_be_inferred_from_an_unrelated_class_or_malformed_text(self):
        for text in (
            "public class Neighbor { public static int staticCall(int input); }",
            "package another; public class InvokeBasic { public static int staticCall(int input); }",
            "package fixture; public class InvokeBasic { public static int staticCall(int input);",
        ):
            with self.subTest(text=text):
                self.capture.side_effect = None
                self.capture.return_value = {"text": text, "issues": [], "exit_code": 0}
                report = self.evaluate([STATIC])
                self.assertEqual(report["metrics"]["body_recovery"]["checks"], {"UNCHECKED": 1})

    def test_same_simple_return_name_cannot_claim_a_missing_renamed_bridge(self):
        case = {"member": "Lexample/FilteredCollection;->spliterator()Ljava/util/Spliterator;",
                "original_code_units": 9}
        selected = {"text": ("package example; class FilteredCollection { "
                             "public final j$.util.Spliterator spliterator(); "
                             "public java.util.Spliterator spliterator$2() { return converted(); } }")}
        self.assertEqual(corpus_eval.recovery_body(case, selected)["status"], "UNCHECKED")
        selected["text"] = selected["text"].replace("j$.util.Spliterator", "java.util.Spliterator")
        self.assertEqual(corpus_eval.recovery_body(case, selected)["status"], "FAIL")

    def test_metric_denominator_keeps_partial_and_fully_measured_methods_distinct(self):
        self.config["methods"][0]["obligations"].append({
            "id": "unsupported-source-call", "kind": "call_presence", "description": "Nonexistent source call",
            "targets": [{"owner": "Ljava/lang/Math;", "name": "min", "descriptor": "(II)I"}],
        })
        self.save_cases()
        report = self.evaluate()
        self.assertEqual(report["metrics"]["corpus_quality"], {
            "checks": {"PASS": 2, "SETUP_ERROR": 1},
            "method_rows": {"fully_measured": 1, "partially_measured": 1},
        })
        self.assertEqual(report["metrics"]["publication"], {
            "checks": {"PASS": 2}, "method_rows": {"fully_measured": 2},
        })

    def test_peer_partial_display_is_unchecked_for_missing_calls_and_is_not_resumed(self):
        publication = {
            "methods": {STATIC: {"at": STATIC, "member": MEMBERS[STATIC], "status": "AVAILABLE",
                                  "text": "public static int staticCall(int input) { return input; }",
                                  "complete": False}},
            "provenance": {"version": "test peer"},
        }
        with patch("jadx.publish", return_value=publication) as publish:
            for _ in range(2):
                report = self.evaluate([STATIC], backend="jadx", resume=True)
                item, = report["inputs"]
                self.assertNotIn("resumed", item)
                row, = item["methods"]
                self.assertEqual({check["metric"]: check["status"] for check in row["checks"]}, {
                    "identity": "PASS", "publication": "UNCHECKED", "body_recovery": "UNCHECKED",
                    "declaration_types": "PASS", "corpus_quality": "UNCHECKED",
                })
            self.assertEqual(publish.call_count, 2)

        publication["methods"][STATIC]["text"] = RECOVERED[STATIC]
        with patch("jadx.publish", return_value=publication):
            report = self.evaluate([STATIC], backend="jadx")
        self.assertEqual(report["metrics"]["publication"]["checks"], {"UNCHECKED": 1})
        self.assertEqual(report["metrics"]["corpus_quality"]["checks"], {"PASS": 1})

    def test_peer_diagnosed_stub_is_a_recovery_gap_with_unknown_semantics(self):
        publication = {
            "methods": {STATIC: {"at": STATIC, "member": MEMBERS[STATIC], "status": "AVAILABLE",
                                  "text": "public static int staticCall(int input) { /* dump */ throw new Error(); }",
                                  "complete": False, "has_body": False, "diagnostics": ["synthetic failure stub"]}},
            "provenance": {"version": "test peer"},
        }
        with patch("jadx.publish", return_value=publication):
            report = self.evaluate([STATIC], backend="jadx")
        self.assertEqual(report["metrics"]["body_recovery"]["checks"], {"FAIL": 1})
        self.assertEqual(report["metrics"]["corpus_quality"]["checks"], {"UNCHECKED": 1})

    def test_checkpoint_requires_same_locators_source_oracle_and_executable(self):
        previous = self.evaluate([STATIC], resume=True)["inputs"][0]
        self.assertTrue(self.evaluate([STATIC], resume=True)["inputs"][0]["resumed"])
        self.capture.assert_called_once()

        expanded = self.evaluate(resume=True)["inputs"][0]
        self.assertNotIn("resumed", expanded)
        self.assertEqual({row["at"] for row in expanded["methods"]}, set(MEMBERS))

        self.write_source(extra=True)
        corpus_assets.prepare_assets(self.project, self.cache, source=self.source)
        repacked = self.evaluate([STATIC], resume=True)["inputs"][0]
        self.assertNotIn("resumed", repacked)
        self.assertNotEqual(repacked["corpus"]["source_archive_sha256"], previous["corpus"]["source_archive_sha256"])
        self.assertEqual(repacked["oracle_sha256"], previous["oracle_sha256"])

        self.config["methods"][0]["obligations"][0]["id"] = "renamed-obligation"
        self.save_cases()
        edited = self.evaluate([STATIC], resume=True)["inputs"][0]
        self.assertNotIn("resumed", edited)
        self.assertNotEqual(edited["oracle_sha256"], repacked["oracle_sha256"])

        self.binary.write_bytes(b"rebuilt producer")
        self.assertNotIn("resumed", self.evaluate([STATIC], resume=True)["inputs"][0])
        self.assertTrue(self.evaluate([STATIC], resume=True)["inputs"][0]["resumed"])

        self.write_apk(self.apk, comment=b"newly pinned packaging")
        self.config["input_sha256"] = evidence.file_digest(self.apk)
        self.project = self.save_project(self.config["input_sha256"])
        self.save_cases()
        corpus_assets.prepare_assets(self.project, self.cache, apk=self.apk)
        changed_apk = self.evaluate([STATIC], resume=True)["inputs"][0]
        self.assertNotIn("resumed", changed_apk)
        self.assertEqual(changed_apk["sha256"], self.config["input_sha256"])
        self.assertNotEqual(changed_apk["sha256"], edited["sha256"])

    def make_model(self, expected):
        directory = self.root / "fixtures" / "calls" / "abs"
        directory.mkdir(parents=True, exist_ok=True)
        (directory / "AbsModel.java").write_text("""public final class AbsModel {
    /* DECBENCH_SELECTED_METHOD */
    public static void main(String[] args) {
        System.out.println("result:" + staticCall(Integer.parseInt(args[0])));
    }
}
""", encoding="utf-8")
        (directory / "model.json").write_text(json.dumps({
            "schema_version": 1, "class_name": "AbsModel", "template": "AbsModel.java",
            "scenarios": [{"id": "negative-input", "arguments": ["-3"], "expected": [expected]}],
        }), encoding="utf-8")
        self.config["methods"][0]["model"] = "calls/abs/model.json"
        self.save_cases()

    @unittest.skipUnless(which("java"), "the modeled behavior integration requires a JDK")
    def test_model_opt_in_and_recovery_compile_error_leave_static_evidence_independent(self):
        self.make_model("result:3")
        with patch("corpus_eval.corpus_models.QUALITY", self.root):
            with patch("corpus_eval.corpus_models.execute", side_effect=AssertionError("model is opt-in")):
                plain = self.evaluate([STATIC], resume=True)
            self.assertNotIn("modeled_behavior", plain["metrics"])

            complete = self.evaluate([STATIC], models=True, resume=True)
            self.assertNotIn("resumed", complete["inputs"][0])
            self.assertEqual(complete["metrics"]["modeled_behavior"], {
                "checks": {"PASS": 1}, "method_rows": {"fully_measured": 1},
            })

            # The call is still visible, though independently executing this publication is unavailable.
            self.capture.side_effect = None
            self.capture.return_value = {
                "text": "public static int staticCall(int input) { return Math.abs(missing); }", "issues": [],
            }
            incomplete = self.evaluate([STATIC], models=True)
            self.assertEqual(incomplete["metrics"]["modeled_behavior"], {
                "checks": {"UNCHECKED": 1}, "method_rows": {"unmeasured": 1},
            })
            self.assertEqual(incomplete["metrics"]["corpus_quality"]["checks"], {"PASS": 1})

            self.make_model("result:-3")
            mismatch = self.evaluate([STATIC], models=True)
            self.assertEqual(mismatch["metrics"]["modeled_behavior"]["checks"], {"SETUP_ERROR": 1})
            self.assertEqual(mismatch["metrics"]["corpus_quality"]["checks"], {"PASS": 1})

    def test_model_withholds_execution_when_pinned_source_is_unavailable(self):
        self.make_model("result:3")
        self.config["methods"][0]["source"]["sha256"] = "0" * 64
        self.save_cases()
        with patch("corpus_eval.corpus_models.QUALITY", self.root), patch(
            "corpus_eval.corpus_models.execute",
            side_effect=AssertionError("execution requires verified source"),
        ):
            report = self.evaluate(models=True)
        self.assertEqual(report["methods_by_status"], {"SETUP_ERROR": 1, "PASS": 1})
        self.assertEqual(report["metrics"]["modeled_behavior"]["checks"], {"SETUP_ERROR": 1})

    def test_model_does_not_execute_a_comment_only_recovery_diagnostic(self):
        self.make_model("result:3")
        self.capture.side_effect = None
        self.capture.return_value = {
            "text": "public static int staticCall(int input) { /* recovery failed */ }",
            "issues": [{"stage": "semantic recovery", "reason": "unavailable region"}],
        }
        with patch("corpus_eval.corpus_models.QUALITY", self.root), patch(
            "corpus_eval.corpus_models.execute", return_value=[(["result:3"], None)],
        ) as execute:
            report = self.evaluate([STATIC], models=True)

        self.assertEqual(execute.call_count, 1)
        self.assertEqual(report["metrics"]["modeled_behavior"]["checks"], {"UNCHECKED": 1})

    def test_bad_unselected_model_leaves_a_scoped_run_and_sibling_available(self):
        model = "calls/abs/wrong.json"
        self.config["methods"][1]["model"] = model
        self.save_cases()
        manifest = self.root / "fixtures" / model
        manifest.parent.mkdir(parents=True)
        with patch("corpus_eval.corpus_models.QUALITY", self.root):
            for contents in (None, "[]"):
                with self.subTest(contents=contents):
                    if contents is not None:
                        manifest.write_text(contents, encoding="utf-8")
                    scoped = self.evaluate([STATIC], models=True)
                    self.assertEqual(scoped["methods_by_status"], {"PASS": 1})
                    self.assertNotIn("modeled_behavior", scoped["metrics"])
                    full = self.evaluate(models=True)
                    self.assertEqual(full["methods_by_status"], {"PASS": 1, "SETUP_ERROR": 1})
                    self.assertEqual(full["metrics"]["modeled_behavior"]["checks"], {"SETUP_ERROR": 1})
                    self.assertEqual(full["metrics"]["corpus_quality"]["checks"], {"PASS": 2})


if __name__ == "__main__":
    unittest.main()
