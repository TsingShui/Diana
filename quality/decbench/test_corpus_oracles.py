"""Independently anchored checks for source-aligned APK oracle evidence."""

from hashlib import sha256
from io import BytesIO
import json
from pathlib import Path
from tarfile import TarInfo, open as open_tar
from tempfile import TemporaryDirectory
import unittest
from zipfile import ZipFile

from corpus_oracles import (
    View, compare, load_cases, read_sources, register_arguments, verify_cases, verify_obligation, verify_source,
)
from evidence import file_digest

SOURCE_PATH = "sample/Example.java"
OWNER = "Lsample/Example;"
MEMBER = OWNER + "->example(Z)V"


def capture(body):
    return "class Example { void example(boolean choice) { " + body + " } }"


def invoke(name, descriptor, pc, owner=OWNER, arguments=None):
    return {
        "name": name, "descriptor": descriptor, "owner": owner, "pc": pc, "arguments": arguments or [],
    }


def case_for(text, mode, original, indexes=None):
    view = View(text)
    bounds = view.method_body(MEMBER)
    obligation = {
        "kind": mode, "id": "modeled-evidence", "description": "Explicit named effects",
        "targets": [{key: event[key] for key in ("owner", "name", "descriptor")} for event in original],
    }
    if indexes is not None:
        obligation["argument_indexes"] = indexes
    case = {"member": MEMBER, "obligations": [obligation]}
    expected = verify_obligation(obligation, case, view, bounds, view.calls(bounds), original)
    if "expected" not in expected or expected.get("status"):
        raise AssertionError(expected)
    return dict(case, verified=[expected])


class CorpusOraclesTest(unittest.TestCase):
    def test_nested_calls_use_evaluation_order_and_ignore_comments_and_sibling_overloads(self):
        case = case_for(capture("write(load());"), "call_sequence",
                        [invoke("load", "()I", 3), invoke("write", "(I)V", 9)])
        same = """
        class Example {
            void example(int choice) { write(3); }
            void example(boolean choice) {
                // write(load());
                int stored = load();
                write(stored);
            }
        }
        """
        result, = compare(case, {"text": same, "issues": [{"kind": "partial"}]})
        self.assertEqual(result["status"], "PASS")
        self.assertEqual([site["pc"] for site in result["expected"]["dex_sites"]], [3, 9])
        result, = compare(case, {"text": capture("write(3); load();")})
        self.assertEqual(result["status"], "FAIL")
        result, = compare(case, {"text": "class Example { void example(boolean choice); }"})
        self.assertEqual(result["status"], "UNCHECKED")

    def test_reordering_exclusive_branches_abstains(self):
        before = capture("if (choice) { first(); } else { second(); }")
        after = capture("if (!choice) { second(); } else { first(); }")
        original = [invoke("first", "()V", 2), invoke("second", "()V", 8)]
        result, = compare(case_for(before, "call_sequence", original), {"text": after})
        self.assertEqual(result["status"], "UNCHECKED")
        self.assertIn("mutually exclusive", result["detail"])

    def test_unrelated_exclusive_arms_do_not_mask_a_separate_order_change(self):
        reference = capture("first(); second(); if (choice) { third(); } else { fourth(); }")
        altered = capture("second(); first(); if (choice) { third(); } else { fourth(); }")
        originals = [invoke(name, "()V", i + 1) for i, name in
                     enumerate(("first", "second", "third", "fourth"))]
        result, = compare(case_for(reference, "call_sequence", originals), {"text": altered})
        self.assertEqual(result["status"], "FAIL")
        self.assertIn("first precedes second", result["detail"])

    def test_duplicate_calls_in_exclusive_arms_can_be_factored(self):
        before = capture("if (choice) { first(); } else { first(); }")
        after = capture("first();")
        original = [invoke("first", "()V", 2), invoke("first", "()V", 8)]
        view = View(before)
        bounds = view.method_body(MEMBER)
        obligation = {"id": "factored-effects", "kind": "call_sequence",
                      "description": "Both call sites", "targets": [invoke("first", "()V", 2)]}
        raw = {"member": MEMBER, "obligations": [obligation]}
        verified = verify_obligation(obligation, raw, view, bounds, view.calls(bounds), original)
        result, = compare(dict(raw, verified=[verified]), {"text": after})
        self.assertEqual(result["status"], "UNCHECKED")

    def test_explicit_transfer_abstains_only_on_order_that_cannot_be_aligned(self):
        source = capture("setup(); text.replace('a', 'b'); done();")
        original = [invoke("setup", "()V", 1), invoke("replace", "(CC)Ljava/lang/String;", 4,
                   "Ljava/lang/String;"), invoke("done", "()V", 8)]
        order = case_for(source, "call_sequence", original)
        argument = case_for(source, "char_arguments", original[1:2])
        recovered = capture("goto retry; done(); retry: setup(); text.replace(97, 98);")
        result, = compare(order, {"text": recovered})
        self.assertEqual(result["status"], "UNCHECKED")
        self.assertIn("explicit control transfers", result["detail"])
        self.assertEqual(result["observed"]["explicit_transfers"], [{"line": 1, "target": "retry"}])
        result, = compare(argument, {"text": recovered})
        self.assertEqual(result["status"], "FAIL")
        self.assertEqual(result["observed"]["argument_types"], [["int", "int"]])

        ordinary = capture('/* goto retry; */ done(); log("goto retry;"); setup(); text.replace(97, 98);')
        result, = compare(order, {"text": ordinary})
        self.assertEqual(result["status"], "FAIL")

    def test_nested_binary_constructor_requires_the_exact_original_owner(self):
        source = capture("new Request.Builder().execute();")
        original = [invoke("<init>", "()V", 1, "Lsample/net/Request$Builder;"),
                    invoke("execute", "()V", 4)]
        for mode in ("call_presence", "call_sequence"):
            case = case_for(source, mode, original)
            for display, status in (
                ("new Request.Builder().execute();", "PASS"),
                ("new sample.net.Request$Builder().execute();", "PASS"),
                ("new Request$Builder().execute();", "UNCHECKED"),
                ("new other.net.Request$Builder().execute();", "UNCHECKED"),
                ("new Different().execute();", "FAIL"),
            ):
                with self.subTest(mode=mode, display=display):
                    result, = compare(case, {"text": capture(display)})
                    self.assertEqual(result["status"], status)
                    if display.startswith("new sample.net"):
                        self.assertEqual(result["observed"]["display_sites"][0]["constructor_spelling"],
                                         "sample.net.Request$Builder")

    def test_char_overload_needs_a_known_type_in_the_selected_scope(self):
        before = capture("char[] values = getValues(); output.append(values[0]);")
        original = [invoke("append", "(C)V", 4, "Lsample/Output;")]
        case = case_for(before, "char_arguments", original)
        for expected, body in (
            ("PASS", "output.append((char) loaded);"),
            ("PASS", "output.append('A');"),
            ("FAIL", "output.append(65);"),
            ("UNCHECKED", "char[] values = getValues(); output.append(other.values[0]);"),
            ("UNCHECKED", "output.append('two');"),
            ("UNCHECKED", "output.append('A'); output.append(3);"),
        ):
            with self.subTest(expected=expected, body=body):
                result, = compare(case, {"text": capture(body)})
                self.assertEqual(result["status"], expected)
        sibling = "class Example { void sibling() { char[] values; } void example(boolean choice) { output.append(values[0]); } }"
        result, = compare(case, {"text": sibling})
        self.assertEqual(result["status"], "UNCHECKED")

    def test_original_argument_words_exclude_receivers_and_skip_wide_slots(self):
        facts = {3: ["integer", 10], 5: ["integer", 8]}
        self.assertEqual(register_arguments(0x406e, 0x5432, 0x6e, "(JI)V", facts),
                         [["integer", 10], ["integer", 8]])
        self.assertEqual(register_arguments(0x0474, 2, 0x74, "(JI)V", facts),
                         [["integer", 10], ["integer", 8]])

    def test_literals_and_static_fields_require_observed_value_witnesses(self):
        before = capture("next(8, CacheControl.FORCE_CACHE);")
        original = [invoke("next", "(ILsample/CacheControl;)V", 4,
                           arguments=[["integer", 8], ["field", "CacheControl", "FORCE_CACHE"]])]
        case = case_for(before, "argument_values", original, [0, 1])
        for expected, body in (
            ("PASS", "next(010, sample.CacheControl.FORCE_CACHE);"),
            ("FAIL", "next(10, CacheControl.FORCE_CACHE);"),
            ("FAIL", "next(8, CacheControl.FORCE_NETWORK);"),
            ("UNCHECKED", "next(value(), CacheControl.FORCE_CACHE);"),
        ):
            with self.subTest(expected=expected, body=body):
                result, = compare(case, {"text": capture(body)})
                self.assertEqual(result["status"], expected)

    def test_duplicate_or_changed_source_member_cannot_establish_oracle(self):
        source = capture("consume();").encode("utf-8")
        method = {"member": MEMBER, "source": {
            "path": SOURCE_PATH, "start_line": 1, "end_line": 1, "sha256": sha256(source).hexdigest(),
        }}
        _, _, sites = verify_source(method, source)
        self.assertEqual([site.name for site in sites], ["consume"])
        with self.assertRaisesRegex(ValueError, "pinned fingerprint"):
            verify_source(method, source + b" ")
        with TemporaryDirectory() as directory:
            tar = Path(directory) / "source.tar.gz"
            with open_tar(tar, "w:gz") as out:
                for name in (SOURCE_PATH, SOURCE_PATH, "sample/Neighbor.java"):
                    header = TarInfo("release/" + name)
                    header.size = len(source)
                    out.addfile(header, BytesIO(source))
            self.assertEqual(read_sources(tar, "release", (SOURCE_PATH, "sample/Neighbor.java")),
                             {SOURCE_PATH: None, "sample/Neighbor.java": source})

    def provenance_fixture(self, directory):
        fixture = Path(__file__).resolve().parents[1] / "fixtures" / "atoms" / "invoke-basic"
        root = Path(directory)
        artifact = root / "input.apk"
        archive = root / "source.tar.gz"
        path = "main/src/main/java/fixture/InvokeBasic.java"
        source = (fixture / "InvokeBasic.java").read_bytes()
        with ZipFile(artifact, "w") as output:
            output.write(fixture / "classes.dex", "classes.dex")
        with open_tar(archive, "w:gz") as output:
            for name, data in ((path, source), ("alternative/" + path, source + b"\x00")):
                entry = TarInfo("release/" + name)
                entry.size = len(data)
                output.addfile(entry, BytesIO(data))
        methods = []
        for identifier, name, start, end, call, target_owner, descriptor in (
            (8, "staticCall", 26, 28, "abs", "Ljava/lang/Math;", "(I)I"),
            (6, "listCall", 46, 50, "size", "Ljava/util/List;", "()I"),
        ):
            methods.append({
                "at": f"e0:l0:c2:m{identifier}", "member": f"Lfixture/InvokeBasic;->{name}(I)I",
                "source": {"path": path, "start_line": start, "end_line": end, "sha256": sha256(source).hexdigest()},
                "obligations": [{
                    "kind": "call_presence", "id": "calls", "description": "The selected invocation",
                    "targets": [{"name": call, "owner": target_owner, "descriptor": descriptor}],
                }],
            })
        return artifact, archive, {"input_sha256": file_digest(artifact), "methods": methods}

    def test_corrupt_source_keeps_original_identity_and_healthy_sibling(self):
        with TemporaryDirectory() as directory:
            artifact, source, cases = self.provenance_fixture(directory)
            first, second = cases["methods"]
            first["source"]["path"] = "alternative/" + first["source"]["path"]
            checked = verify_cases(cases, artifact, source, "release")
        failed, healthy = [checked[item["at"]] for item in (first, second)]
        self.assertTrue(failed["original_verified"])
        self.assertFalse(failed["source_verified"])
        self.assertNotIn("original_error", failed)
        self.assertIn("fingerprint", failed["source_error"])
        self.assertEqual(failed["verified"][0]["status"], "SETUP_ERROR")
        self.assertTrue(healthy["original_verified"])
        self.assertTrue(healthy["source_verified"])
        self.assertIn("expected", healthy["verified"][0])

    def test_wrong_physical_locator_keeps_source_and_healthy_sibling(self):
        with TemporaryDirectory() as directory:
            artifact, source, cases = self.provenance_fixture(directory)
            first, second = cases["methods"]
            first["at"] = "e0:l0:c2:m7"
            checked = verify_cases(cases, artifact, source, "release")
        failed, healthy = [checked[item["at"]] for item in (first, second)]
        self.assertFalse(failed["original_verified"])
        self.assertTrue(failed["source_verified"])
        self.assertNotIn("source_error", failed)
        self.assertIn("locator and descriptor", failed["original_error"])
        self.assertEqual(failed["verified"][0]["status"], "SETUP_ERROR")
        self.assertTrue(healthy["original_verified"])
        self.assertTrue(healthy["source_verified"])
        self.assertIn("expected", healthy["verified"][0])

    def test_pinned_apk_mismatch_is_a_setup_error(self):
        with TemporaryDirectory() as directory:
            artifact, source, cases = self.provenance_fixture(directory)
            cases["input_sha256"] = "0" * 64
            checked = verify_cases(cases, artifact, source, "release")
        for case in checked.values():
            self.assertFalse(case["original_verified"])
            self.assertTrue(case["source_verified"])
            self.assertIn("pinned", case["original_error"])
            result, = compare(case, {"text": capture("consume();")})
            self.assertEqual(result["status"], "SETUP_ERROR")
            self.assertIn("pinned", result["detail"])

    def test_manifest_accepts_legal_array_call_owners(self):
        targets = [{"owner": "[I", "name": "clone", "descriptor": "()Ljava/lang/Object;"}]
        manifest = {
            "schema_version": 1, "input_sha256": "0" * 64, "source_revision": "0" * 40,
            "methods": [{
                "at": "e0:l0:c0:m0", "member": MEMBER,
                "source": {"path": SOURCE_PATH, "start_line": 1, "end_line": 1, "sha256": "0" * 64},
                "obligations": [{"id": "array-clone", "kind": "call_presence",
                                 "description": "Preserve array clone", "targets": targets}],
            }],
        }
        with TemporaryDirectory() as directory:
            path = Path(directory) / "cases.json"
            for owner in ("[I", "[Ljava/lang/String;", "[[Lsample/Outer$Element;", "[Lsample/名称;", "[" * 255 + "I"):
                with self.subTest(owner=owner):
                    targets[0]["owner"] = owner
                    path.write_text(json.dumps(manifest))
                    self.assertEqual(load_cases(path), manifest)
            for owner in ("[", "[V", "[II", "I", "[Ljava/lang/String", "[L;", "[Lsample//Element;",
                          "[Lsample/;", "[Lsample/Bad+Name;", "[Lsample/Bad\nName;", "[" * 256 + "I"):
                with self.subTest(owner=owner):
                    targets[0]["owner"] = owner
                    path.write_text(json.dumps(manifest))
                    with self.assertRaisesRegex(ValueError, "invalid call owner"):
                        load_cases(path)

    def test_manifest_pins_distinct_original_methods(self):
        path = Path(__file__).with_name("corpora") / "antennapod-3.12.3.cases.json"
        cases = load_cases(path)
        self.assertEqual(len(cases["methods"]), 14)
        for method in cases["methods"]:
            source = method["source"]
            self.assertIn(cases["source_revision"], source["url"])
            self.assertIn(f"#L{source['start_line']}-L{source['end_line']}", source["url"])


if __name__ == "__main__":
    unittest.main()
