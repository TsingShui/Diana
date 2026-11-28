"""Check physical DEX selection and a real JADX publication when a JAR is provided."""

import json
import os
from pathlib import Path
import subprocess
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import patch
import warnings
from zipfile import ZipFile

import jadx
from dex_truth import methods

FIXTURES = Path(__file__).resolve().parents[1] / "fixtures"
SCALARS = FIXTURES / "atoms/scalar-use-views/classes.dex"
OWNER = "Lfixture/ScalarViews;"
ARRAY = {"at": "e0:l0:c0:m0", "member": OWNER + "->array([C)Ljava/lang/String;"}
CARRIER = {"at": "e0:l0:c0:m2", "member": OWNER + "->carrier(Ljava/lang/String;)I"}
JADX_JAR = os.environ.get("DECBENCH_JADX_JAR")
NEWPIPE_APK = os.environ.get("DECBENCH_NEWPIPE_APK")


class JadxSelectionTest(unittest.TestCase):
    def test_checks_physical_apk_entry_class_and_overload(self):
        with TemporaryDirectory() as scratch:
            artifact = Path(scratch) / "scalar.apk"
            with ZipFile(artifact, "w") as archive:
                archive.writestr("other.txt", "non-DEX entry")
                archive.write(SCALARS, "classes2.dex")
            actual = {"at": "e1:l0:c0:m7", "member": OWNER + "->probe(C)I"}
            wrong_descriptor = {"at": "e1:l0:c0:m8", "member": OWNER + "->probe(C)I"}
            wrong_entry = {"at": "e0:l0:c0:m7", "member": actual["member"]}
            results = {}
            requests = jadx.selected_requests(artifact, {
                selection["at"]: selection for selection in (actual, wrong_descriptor, wrong_entry)
            }, results)
        self.assertEqual(len(requests), 1)
        self.assertEqual(requests[0]["at"], actual["at"])
        self.assertEqual(requests[0]["physical"]["owner"], OWNER)
        self.assertEqual(requests[0]["physical"]["input_name"], "classes2.dex")
        self.assertGreater(requests[0]["physical"]["offset"], 0)
        self.assertFalse(results[wrong_descriptor["at"]]["identified"])
        self.assertFalse(results[wrong_entry["at"]]["identified"])

    def test_duplicate_archive_entry_names_are_unavailable(self):
        with TemporaryDirectory() as scratch:
            artifact = Path(scratch) / "duplicate.apk"
            with ZipFile(artifact, "w") as archive, warnings.catch_warnings():
                warnings.simplefilter("ignore", UserWarning)
                archive.write(SCALARS, "classes.dex")
                archive.write(SCALARS, "classes.dex")
            results = {}
            requests = jadx.selected_requests(artifact, {CARRIER["at"]: CARRIER}, results)
        self.assertEqual(requests, [])
        self.assertIn("unique root DEX", results[CARRIER["at"]]["diagnostics"][0])

    def test_timeout_keeps_published_method_and_raw_diagnostics(self):
        def timeout(argv, work, seconds):
            document = {
                **ARRAY, "status": "AVAILABLE", "complete": False, "has_body": True, "identified": True,
                "text": "incomplete body", "source_span": {"start_utf16": 0, "definition_utf16": 0, "end_utf16": 15},
                "diagnostics": ["decompiler failure detail"],
            }
            Path(argv[-1]).write_text(json.dumps({"version": "1.5.7"}) + "\n" + json.dumps(document) + "\n{")
            raise subprocess.TimeoutExpired(argv, seconds, output=b"raw progress", stderr=b"raw problem")
        with TemporaryDirectory() as scratch, patch.object(jadx, "run", side_effect=timeout) as run:
            jar = Path(scratch) / "fake.jar"
            jar.write_bytes(b"jar identity")
            report = jadx.publish(jar, SCALARS, [ARRAY, CARRIER], 4)
        run.assert_called_once()
        self.assertEqual(report["provenance"]["version"], "1.5.7")
        self.assertEqual(report["worker"]["status"], "TIMED_OUT")
        self.assertEqual(report["stdout"], "raw progress")
        self.assertEqual(report["stderr"], "raw problem")
        self.assertEqual(report["methods"][ARRAY["at"]]["text"], "incomplete body")
        self.assertEqual(report["methods"][ARRAY["at"]]["diagnostics"], ["decompiler failure detail"])
        self.assertFalse(report["methods"][ARRAY["at"]].get("setup_error"))
        self.assertEqual(report["methods"][CARRIER["at"]]["status"], "UNAVAILABLE")
        self.assertTrue(report["methods"][CARRIER["at"]]["setup_error"])
        self.assertIn("timed out", report["methods"][CARRIER["at"]]["diagnostics"][0])


    def test_missing_launcher_is_setup_failure(self):
        with TemporaryDirectory() as scratch, patch.object(jadx, "run", side_effect=OSError("launcher unavailable")):
            jar = Path(scratch) / "jadx.jar"
            jar.write_bytes(b"jar identity")
            report = jadx.publish(jar, SCALARS, [ARRAY], 4)
        self.assertEqual(report["worker"], {"status": "SETUP_ERROR", "exit_code": None,
                                            "error": "launcher unavailable"})
        self.assertTrue(report["methods"][ARRAY["at"]]["setup_error"])

    def test_successful_exit_with_missing_protocol_is_setup_failure(self):
        worker = subprocess.CompletedProcess(["java"], 0, "progress", "")
        with TemporaryDirectory() as scratch, patch.object(jadx, "run", return_value=worker):
            jar = Path(scratch) / "jadx.jar"
            jar.write_bytes(b"jar identity")
            report = jadx.publish(jar, SCALARS, [ARRAY], 4)
        self.assertEqual(report["worker"]["status"], "PROTOCOL_ERROR")
        self.assertEqual(report["worker"]["exit_code"], 0)
        self.assertTrue(report["methods"][ARRAY["at"]]["setup_error"])


@unittest.skipUnless(JADX_JAR, "set DECBENCH_JADX_JAR to the official JADX fat JAR")
class ActualJadxTest(unittest.TestCase):
    def test_overloads_are_selected_by_original_identity_and_text_is_unmodified(self):
        char = {"at": "e0:l0:c0:m7", "member": OWNER + "->probe(C)I"}
        integer = {"at": "e0:l0:c0:m8", "member": OWNER + "->probe(I)I"}
        report = jadx.publish(Path(JADX_JAR), SCALARS, [char, integer, ARRAY, CARRIER], 30)
        self.assertEqual(report["diagnostics"], [], report["stderr"])
        self.assertEqual(list(report["methods"]), [item["at"] for item in [char, integer, ARRAY, CARRIER]])
        source = report["classes"]["e0:l0:c0"].encode("utf-16-le")
        for row in report["methods"].values():
            self.assertTrue(row["identified"] and row["complete"] and row["has_body"], row)
            span = row["source_span"]
            self.assertEqual(row["text"], source[2 * span["start_utf16"]:2 * span["end_utf16"]].decode("utf-16-le"))
        self.assertIn("probe(char ", report["methods"][char["at"]]["text"])
        self.assertIn("probe(int ", report["methods"][integer["at"]]["text"])
        self.assertIn("probe((int)", report["methods"][CARRIER["at"]]["text"])
        self.assertIn("append((int)", report["methods"][ARRAY["at"]]["text"])

    def test_original_native_declaration_is_available_without_claiming_a_body(self):
        artifact = FIXTURES / "atoms/invoke-word-order/classes.dex"
        at, member = next((at, member) for at, member in methods(artifact) if member.endswith("->take(JI)I"))
        report = jadx.publish(Path(JADX_JAR), artifact, [{"at": at, "member": member}], 30)
        row = report["methods"][at]
        self.assertEqual(row["status"], "AVAILABLE", (row, report["stderr"]))
        self.assertTrue(row["identified"] and row["complete"])
        self.assertFalse(row["has_body"])
        self.assertTrue(row["text"].strip().endswith(";"))

    def test_bridge_metadata_is_attributed_to_the_concrete_original_method(self):
        artifact = FIXTURES / "classes/class-bridge/classes.dex"
        owner = "Lfixture/ClassBridge$Box;"
        concrete = {"at": "e0:l0:c0:m2", "member": owner + "->compare(Ljava/lang/Comparable;Ljava/lang/Comparable;)I"}
        bridge = {"at": "e0:l0:c0:m3", "member": owner + "->compare(Ljava/lang/Object;Ljava/lang/Object;)I"}
        report = jadx.publish(Path(JADX_JAR), artifact, [bridge, concrete], 30)
        self.assertEqual(report["diagnostics"], [], report["stderr"])
        original = report["methods"][concrete["at"]]
        self.assertEqual(original["status"], "AVAILABLE", original)
        self.assertIn("return left.compareTo(right);", original["text"])
        self.assertIn("synthetic bridge", original["diagnostics"][0])
        omitted_bridge = report["methods"][bridge["at"]]
        self.assertEqual(omitted_bridge["status"], "UNAVAILABLE", omitted_bridge)
        self.assertTrue(omitted_bridge["identified"])
        self.assertIsNone(omitted_bridge["text"])
        self.assertFalse(omitted_bridge.get("setup_error"))
        self.assertEqual(report["worker"]["status"], "COMPLETE")

    @unittest.skipUnless(NEWPIPE_APK, "set DECBENCH_NEWPIPE_APK to the pinned NewPipe 0.28.8 APK")
    def test_real_fallback_dump_is_identified_but_incomplete(self):
        at = "e4:l0:c7681:m56156"
        member = "Lorg/schabi/newpipe/streams/io/StoredFileHelper;->create()Z"
        report = jadx.publish(Path(JADX_JAR), Path(NEWPIPE_APK), [{"at": at, "member": member}], 45)
        row = report["methods"][at]
        self.assertEqual(report["worker"]["status"], "COMPLETE", report["stderr"])
        self.assertEqual(row["status"], "AVAILABLE", row)
        self.assertTrue(row["identified"])
        self.assertFalse(row["complete"])
        self.assertFalse(row["has_body"])
        self.assertFalse(row.get("setup_error"))
        self.assertIn("instruction dump", row["diagnostics"][0])
        self.assertIn("L78:", row["text"])
        self.assertIn("Method not decompiled: org.schabi.newpipe.streams.io.StoredFileHelper.create():boolean",
                      row["text"])
