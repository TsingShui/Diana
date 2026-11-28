"""Exercise the two versioned legacy publications at the subprocess boundary."""

from pathlib import Path
import sys
import unittest
from unittest.mock import patch

import legacy_diana


BINARY = Path("diana-early")
APK = Path("fixture.apk")
OWNER = "Lfixture/Sample;"
AT = "e4:l0:c12:m34"
MEMBER = f"{OWNER}->choose([JI)C"
TEXT = "char choose(long[] words, int index) { return 'x'; }"


def legacy_inventory(items, exit_code=0, **fields):
    return {
        "exit_code": exit_code, "publication": {
            "schema_version": 1, "command": "methods", "items": items, **fields,
        },
    }


def legacy_method(kind="ready", exit_code=0, issues=None):
    product = {"text": TEXT, "issues": [] if issues is None else issues}
    if kind == "stopped":
        product["error"] = "render capacity"
    return {"exit_code": exit_code, "publication": {
        "schema_version": 3, "command": "get-method", "selection": {
            "entry": 4, "logical": 0, "class": {"class_def": 12}, "method": 34,
            "class_at": f"e4:l0:c12::{OWNER}", "method_at": f"{AT}::{MEMBER}",
        },
        "result": {"kind": kind, "product": product},
    }}


class LegacyDianaTest(unittest.TestCase):
    def test_fallback_requires_the_explicit_unsupported_flag_error(self):
        capture = {"exit_code": 2, "argv": ["diana", "methods", "fixture.apk", "--no-limit"],
                   "stderr": "error: unexpected argument '--no-limit' found\n\nUsage: diana methods <INPUT>"}
        self.assertTrue(legacy_diana.unsupported_unlimited(capture))
        for other in (
            {"exit_code": 1}, {"stderr": "error: failed to read input"},
            {"stderr": "error: unexpected argument '--jobs' found"}, {"publication": {}},
            {"argv": ["diana", "methods", "fixture.apk"]},
        ):
            with self.subTest(other=other):
                self.assertFalse(legacy_diana.unsupported_unlimited({**capture, **other}))

    def test_complete_inventory_splits_combined_identity_without_limiting_the_query(self):
        other = "e4:l0:c12:m35"
        constructor = f"{OWNER}-><init>()V"
        reply = legacy_inventory([{"at": f"{AT}::{MEMBER}"}, {"at": f"{other}::{constructor}"}])
        with patch("legacy_diana.run_cli", return_value=reply) as run:
            members, capture = legacy_diana.inventory(BINARY, APK, 12)
        self.assertEqual(members, {(AT, MEMBER), (other, constructor)})
        self.assertTrue(capture["complete"])
        self.assertEqual(capture["argv"], [str(BINARY), "methods", str(APK), "--format", "json"])
        run.assert_called_once_with(capture["argv"], 12, legacy_diana.MAX_INVENTORY_BYTES)

    def test_incomplete_bad_or_duplicate_inventory_does_not_certify_absence(self):
        original = {"at": f"{AT}::{MEMBER}"}
        for reply in (
            legacy_inventory([original], exit_code=1),
            legacy_inventory([original], complete=False),
            legacy_inventory([original, original]),
            legacy_inventory([original, {"at": f"{AT}::{OWNER}->other()V"}]),
            legacy_inventory([original, {"at": AT, "member": MEMBER}]),
            legacy_inventory([original, {"at": f"{AT}::{OWNER}->other(V)Q"}]),
            legacy_inventory([original], schema_version=2),
        ):
            with self.subTest(reply=reply), patch("legacy_diana.run_cli", return_value=reply):
                members, capture = legacy_diana.inventory(BINARY, APK, 12)
                self.assertEqual(members, set())
                self.assertFalse(capture["complete"])
                self.assertIn("error", capture)

    def test_ready_schema_three_preserves_clean_and_local_issue_publications(self):
        for issues in ([], [{"method": 34, "detail": "unavailable expression", "kind": "Unsupported"}]):
            with self.subTest(issues=issues), patch("legacy_diana.run_cli", return_value=legacy_method(issues=issues)) as run:
                observed = legacy_diana.method(BINARY, APK, AT, 12, member=MEMBER)
                self.assertEqual(observed["text"], TEXT)
                self.assertEqual(observed["issues"], issues)
                self.assertNotIn("error", observed)
                run.assert_called_once_with(
                    [str(BINARY), "get-method", str(APK), f"{AT}::{MEMBER}", "--format", "json"],
                    12, legacy_diana.MAX_METHOD_BYTES,
                )

    def test_classified_render_stop_keeps_an_explicitly_incomplete_prefix(self):
        reply = legacy_method(kind="stopped", exit_code=1)
        with patch("legacy_diana.run_cli", return_value=reply):
            observed = legacy_diana.method(BINARY, APK, AT, 12, member=MEMBER)
        self.assertEqual(observed["exit_code"], 1)
        self.assertEqual(observed["text"], TEXT)
        self.assertIn("incomplete prefix", observed["error"])

    def test_response_must_attest_to_identity_and_match_historical_publication_status(self):
        for change in ("selector", "physical-entry", "method-index", "descriptor", "schema", "flat-result", "exit"):
            with self.subTest(change=change):
                reply = legacy_method()
                document = reply["publication"]
                if change == "selector":
                    document["selection"]["method_at"] = AT
                elif change == "physical-entry":
                    document["selection"]["entry"] = 7
                elif change == "method-index":
                    document["selection"]["method"] = 35
                elif change == "descriptor":
                    document["selection"]["method_at"] = f"{AT}::{OWNER}->choose([JJ)C"
                elif change == "schema":
                    document["schema_version"] = 5
                elif change == "flat-result":
                    document["result"] = {"text": TEXT}
                else:
                    reply["exit_code"] = 1
                with patch("legacy_diana.run_cli", return_value=reply):
                    observed = legacy_diana.method(BINARY, APK, AT, 12, member=MEMBER)
                self.assertIn("error", observed)
                self.assertNotIn("text", observed)

    def test_subprocess_capture_discards_oversized_publication_and_times_out(self):
        oversized = legacy_diana.run_cli([sys.executable, "-c", "print('a' * 65536)"], 5, 128)
        self.assertIn("byte bound", oversized["error"])
        self.assertNotIn("publication", oversized)
        timed_out = legacy_diana.run_cli([sys.executable, "-c", "import time; time.sleep(2)"], 0.05, 128)
        self.assertIsNone(timed_out["exit_code"])
        self.assertIn("timed out", timed_out["error"])


if __name__ == "__main__":
    unittest.main()
