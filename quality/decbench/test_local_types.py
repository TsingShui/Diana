"""Check independent debug-local evidence, matching and type differences."""

from __future__ import annotations

import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

import local_types

OWNER = "Lfixture/jvm/SourceRecovery;"
AT = "j0:m4"
MEMBER = OWNER + "->descendingSum(I)I"
SOURCE = Path(__file__).resolve().parents[1] / "fixtures/jvm/source-recovery/SourceRecovery.java"
MANIFEST = {
    "local_types": {"class_at": "j0", "class_member": OWNER},
    "methods": [{
        "at": AT, "member": MEMBER,
        "local_types": [{
            "id": "accumulator",
            "original": {"name": "total", "slot": 1},
            "recovered": {"kind": "returned-local"},
        }],
    }],
}


class MatchingTest(unittest.TestCase):
    def test_named_slot_intervals_remain_distinct(self):
        records = [
            {"name": "x", "slot": 1, "start_pc": 3, "descriptor": "I"},
            {"name": "x", "slot": 1, "start_pc": 15, "descriptor": "J"},
        ]
        with self.assertRaisesRegex(local_types.Unsupported, "ambiguous"):
            local_types.original_local(records, {"name": "x", "slot": 1})
        selected = local_types.original_local(records, {"name": "x", "slot": 1, "start_pc": 15})
        self.assertEqual(selected["descriptor"], "J")

    def test_alias_follows_known_imports_and_declines_shadowing(self):
        aliases = {"type_aliases": {"Value": "Ljvm/Value;", "different.Value": "Ldifferent/Value;"}}
        self.assertEqual(local_types.resolve_type("Value[]", aliases, [], set()), "[Ljvm/Value;")
        self.assertEqual(
            local_types.resolve_type("Value", aliases, ["different.Value"], set()), "Ldifferent/Value;"
        )
        self.assertIsNone(local_types.resolve_type("Value", aliases, [], {"Value"}))
        self.assertIsNone(local_types.resolve_type("Value", aliases, ["different.*"], set()))
        self.assertIsNone(local_types.resolve_type("var", aliases, [], set()))

    def test_return_role_excludes_parameters_and_ambiguous_local_names(self):
        method = {
            "return_values": ["result", "result"],
            "variables": [{"name": "result", "kind": "local", "spelling": "int"}],
        }
        self.assertEqual(local_types.recovered_local(method, {"kind": "returned-local"})["spelling"], "int")
        method["variables"].append({"name": "result", "kind": "local", "spelling": "short"})
        with self.assertRaisesRegex(local_types.Unsupported, "ambiguous"):
            local_types.recovered_local(method, {"kind": "returned-local"})
        method["variables"] = [{"name": "result", "kind": "parameter", "spelling": "int"}]
        with self.assertRaisesRegex(local_types.Unsupported, "ambiguous"):
            local_types.recovered_local(method, {"kind": "returned-local"})


@unittest.skipUnless(os.name == "posix" and shutil.which("java"), "needs a POSIX launcher and JDK")
class JdkLocalTypesTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.scratch = tempfile.TemporaryDirectory()
        cls.addClassCleanup(cls.scratch.cleanup)
        cls.root = Path(cls.scratch.name)
        for variant, option in (("debug", "-g"), ("stripped", "-g:none")):
            subprocess.run(
                ["java", "-m", "jdk.compiler/com.sun.tools.javac.Main", option,
                 "-encoding", "UTF-8", "-d", str(cls.root / variant), str(SOURCE)],
                check=True, capture_output=True, timeout=15,
            )
        cls.reference = SOURCE.read_text(encoding="utf-8")

    def artifact(self, variant: str) -> Path:
        return self.root / variant / "fixture/jvm/SourceRecovery.class"

    def launcher(self, recovered: str) -> Path:
        document = {
            "schema_version": 5, "command": "get-method",
            "selection": {"class": OWNER, "class_at": "j0", "method_at": AT},
            "result": {"methods": [{"at": AT, "name": "descendingSum", "prototype": "(I)I"}],
                       "text": recovered},
        }
        gate = self.root / "decoder"
        gate.write_text(
            f"#!{sys.executable}\nimport sys\n"
            f"assert sys.argv[1] == 'get-method' and sys.argv[3] == {AT!r}\n"
            f"print({json.dumps(document)!r})\n",
            encoding="utf-8",
        )
        gate.chmod(0o755)
        return gate

    def test_real_debug_and_stripped_local_tables(self):
        owner, methods = local_types.read_methods(self.artifact("debug").read_bytes(), {4})
        self.assertEqual(owner, OWNER)
        self.assertEqual(methods[4][0], MEMBER)
        record = local_types.original_local(methods[4][1], {"name": "total", "slot": 1})
        self.assertEqual(record["descriptor"], "I")
        self.assertLess(record["start_pc"], record["end_pc"])
        stripped = local_types.compare(self.root / "unavailable", self.artifact("stripped"), MANIFEST, 15)
        self.assertEqual(stripped[AT][0]["status"], "UNCHECKED")
        self.assertIn("LocalVariableTable is absent", stripped[AT][0]["detail"])

    def test_independent_comparison_accepts_renaming_and_detects_widened_local(self):
        renamed = self.reference.replace("total", "accumulated")
        passing = local_types.compare(self.launcher(renamed), self.artifact("debug"), MANIFEST, 15)
        self.assertEqual(passing[AT][0]["status"], "PASS")
        self.assertEqual(passing[AT][0]["observed"]["name"], "accumulated")
        widened = renamed.replace("int accumulated", "long accumulated").replace(
            "return accumulated;", "return (int) accumulated;"
        )
        failing = local_types.compare(self.launcher(widened), self.artifact("debug"), MANIFEST, 15)
        self.assertEqual(failing[AT][0]["status"], "FAIL")
        self.assertEqual(failing[AT][0]["expected"]["descriptor"], "I")
        self.assertEqual(failing[AT][0]["observed"]["descriptor"], "J")

    def test_unresolved_syntax_keeps_independent_debug_evidence(self):
        unknown = self.reference.replace("int total", "var total")
        result = local_types.compare(self.launcher(unknown), self.artifact("debug"), MANIFEST, 15)
        self.assertEqual(result[AT][0]["status"], "UNCHECKED")
        self.assertEqual(result[AT][0]["expected"]["descriptor"], "I")
        broken = self.reference.replace("int total = 0", "int total = ???")
        result = local_types.compare(self.launcher(broken), self.artifact("debug"), MANIFEST, 15)
        self.assertEqual(result[AT][0]["status"], "UNCHECKED")
        self.assertIn("syntax", result[AT][0]["detail"])

    def test_truncation_is_a_class_format_error(self):
        data = self.artifact("debug").read_bytes()
        with self.assertRaises(local_types.ClassFormat):
            local_types.read_methods(data[:-1], {4})
