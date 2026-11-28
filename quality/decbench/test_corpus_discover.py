"""Check how a public APK method is independently selected for source evidence."""

from io import BytesIO
import json
from pathlib import Path
from tarfile import TarInfo, open as open_tar
from tempfile import TemporaryDirectory
import unittest
from zipfile import ZipFile

from corpus_discover import (
    SourceFile, balanced_cases, candidate_targets, discover, invocation_arguments, method_declarations,
    source_environment, source_files, source_match,
)
from corpus_oracles import Site, View, compare, load_cases, verify_cases
from evidence import file_digest

FIXTURE = Path(__file__).resolve().parents[1] / "fixtures" / "atoms" / "invoke-basic"
REVISION = "1" * 40
REPOSITORY = "https://github.com/example/project"


def source_archive(path, sources):
    with open_tar(path, "w:gz") as output:
        for relative, data in sources:
            entry = TarInfo("release/" + relative)
            entry.size = len(data)
            output.addfile(entry, BytesIO(data))


def project_apk(path, *, duplicate=False):
    with ZipFile(path, "w") as output:
        output.write(FIXTURE / "classes.dex", "classes.dex")
        if duplicate:
            output.write(FIXTURE / "classes.dex", "classes2.dex")


def analyze(text):
    view = View(text)
    source = SourceFile("app/src/main/java/example/Example.java", "app", "Lexample/Example;", text.encode())
    environment = source_environment(view, source.owner)
    return method_declarations(view, environment[-1]), environment


class DiscoverTest(unittest.TestCase):
    def test_full_prototype_checks_imports_return_arrays_and_overloads(self):
        declarations, environment = analyze("""package example;
            import expected.Reference;
            class Example {
                Reference[] selected(Reference input) { return copy(input); }
                Reference[] selected(String input) { return convert(input); }
                Reference selected(int input) { return convert(input); }
                <T> T generic(T input) { return produce(input); }
                static class Nested {
                    Reference[] selected(Reference input) { return incorrect(input); }
                }
            }""")
        methods = [method.name for method in declarations]
        self.assertEqual(methods, ["selected", "selected", "selected"])
        for signature in (
            "selected(Lexpected/Reference;)[Lexpected/Reference;",
            "selected(Ljava/lang/String;)[Lexpected/Reference;",
            "selected(I)Lexpected/Reference;",
        ):
            with self.subTest(signature=signature):
                match = source_match(declarations, "Lexample/Example;->" + signature, environment, set())
                self.assertIsNotNone(match)
        for signature in (
            "selected(Lwrong/Reference;)[Lexpected/Reference;",
            "selected(Lexpected/Reference;)Lexpected/Reference;",
            "selected(Lexpected/Reference;)[Lwrong/Reference;",
            "selected(J)Lexpected/Reference;",
        ):
            with self.subTest(signature=signature):
                self.assertIsNone(source_match(declarations, "Lexample/Example;->" + signature, environment, set()))

    def test_wildcard_and_locally_shadowed_type_are_not_assigned_guessed_prototypes(self):
        declarations, environment = analyze("""package example;
            import absent.*;
            class Example {
                String selected(String input) { return copy(input); }
                Hidden selected(Hidden input) { return copy(input); }
                class Hidden {}
            }""")
        owner = "Lexample/Example;->"
        self.assertIsNone(source_match(declarations, owner + "selected(Ljava/lang/String;)Ljava/lang/String;",
                                       environment, set()))
        self.assertIsNone(source_match(declarations, owner + "selected(Lexample/Hidden;)Lexample/Hidden;",
                                       environment, {"Lexample/Hidden;"}))

    def test_generated_targets_need_unique_sites_on_both_sides(self):
        source = [
            Site("ambiguousInDex", (), 1), Site("ambiguousInSource", (), 1), Site("ambiguousInSource", (), 2),
            Site("selected", (("value",),), 3),
        ]
        dex = [
            {"owner": "Lraw/A;", "name": "ambiguousInDex", "descriptor": "()V"},
            {"owner": "Lraw/B;", "name": "ambiguousInDex", "descriptor": "()V"},
            {"owner": "Lraw/B;", "name": "ambiguousInSource", "descriptor": "()V"},
            {"owner": "Lraw/B;", "name": "selected", "descriptor": "(I)V"},
            {"owner": "Lraw/B;", "name": "selected", "descriptor": "()J"},
        ]
        self.assertEqual(candidate_targets(source, dex), [dex[3]])

    def test_round_robin_spreads_a_budget_deterministically_across_modules_and_owners(self):
        items = [
            {"module": "b", "at": "6", "member": "LOne;->a()V"},
            {"module": "b", "at": "7", "member": "LOne;->b()V"},
            {"module": "a", "at": "1", "member": "LOne;->a()V"},
            {"module": "a", "at": "2", "member": "LOne;->b()V"},
            {"module": "a", "at": "3", "member": "LTwo;->a()V"},
        ]
        order = [item["at"] for item in balanced_cases(items)]
        self.assertEqual(order, ["1", "6", "3", "7", "2"])
        self.assertEqual([item["at"] for item in balanced_cases(reversed(items))], order)

    def test_argument_evidence_requires_agreement_with_original_operands(self):
        member = "Lsample/Example;->run()V"
        text = "class Example { void run() { use('X'); count(8); other(19); } }"
        view = View(text)
        bounds = view.method_body(member)
        sites = view.calls(bounds)
        dex = [
            {"owner": "Lsample/Example;", "name": "use", "descriptor": "(C)V", "pc": 2, "arguments": [None]},
            {"owner": "Lsample/Example;", "name": "count", "descriptor": "(I)V", "pc": 5,
             "arguments": [["integer", 8]]},
            {"owner": "Lsample/Example;", "name": "other", "descriptor": "(I)V", "pc": 9,
             "arguments": [["integer", 11]]},
        ]
        case = {"member": member, "obligations": []}
        invocation_arguments(case, candidate_targets(sites, dex), view, bounds, sites, dex)
        self.assertEqual([(item["kind"], item["targets"][0]["name"]) for item in case["obligations"]],
                         [("argument_values", "count"), ("char_arguments", "use")])

    def test_discovers_a_real_method_then_rejects_duplicate_original_owners(self):
        with TemporaryDirectory() as directory:
            scratch = Path(directory)
            source = scratch / "source.tar.gz"
            artifact = scratch / "project.apk"
            project_apk(artifact)
            path = "main/src/main/java/fixture/InvokeBasic.java"
            source_archive(source, [(path, (FIXTURE / "InvokeBasic.java").read_bytes())])
            result = discover(artifact, source, "release", REVISION, REPOSITORY, limit=2)
            self.assertTrue(result["methods"])
            members = {method["member"]: method for method in result["methods"]}
            member = "Lfixture/InvokeBasic;->staticCall(I)I"
            self.assertIn(member, members)
            method = members[member]
            self.assertEqual(method["source"]["path"], path)
            self.assertEqual(method["module"], "main")
            expected, = method["obligations"][0]["targets"]
            self.assertEqual(expected, {"owner": "Ljava/lang/Math;", "name": "abs", "descriptor": "(I)I"})
            persisted = scratch / "cases.json"
            persisted.write_text(json.dumps(result))
            run = verify_cases(load_cases(persisted), artifact, source, "release")
            self.assertTrue(all("expected" in case["verified"][0] for case in run.values()))
            good = "static int staticCall(int value) { return Math.abs(value); }"
            changed = "static int staticCall(int value) { return Math.abs(value, 0); }"
            self.assertEqual(compare(run[method["at"]], {"text": good})[0]["status"], "PASS")
            self.assertEqual(compare(run[method["at"]], {"text": changed})[0]["status"], "FAIL")
            self.assertEqual(
                discover(artifact, source, "release", REVISION, REPOSITORY, limit=2,
                         input_sha256=file_digest(artifact)), result)
            full = discover(artifact, source, "release", REVISION, REPOSITORY)
            partitions = [discover(artifact, source, "release", REVISION, REPOSITORY, shard=(index, 2))
                          for index in (1, 2)]
            split = [{case["at"] for case in part["methods"]} for part in partitions]
            self.assertFalse(split[0] & split[1])
            self.assertEqual(split[0] | split[1], {case["at"] for case in full["methods"]})
            with self.assertRaisesRegex(ValueError, "pinned discovery input"):
                discover(artifact, source, "release", REVISION, REPOSITORY, input_sha256="0" * 64)

            project_apk(artifact, duplicate=True)
            self.assertEqual(discover(artifact, source, "release", REVISION, REPOSITORY)["methods"], [])

    def test_array_clone_discovery_emits_a_manifest_that_loads_and_verifies(self):
        fixture = FIXTURE.parents[1] / "algorithms" / "algo-sort"
        with TemporaryDirectory() as directory:
            scratch = Path(directory)
            source = scratch / "source.tar.gz"
            artifact = scratch / "project.apk"
            with ZipFile(artifact, "w") as output:
                output.write(fixture / "classes.dex", "classes.dex")
            source_archive(source, [
                ("app/src/main/java/fixture/AlgoSort.java", (fixture / "AlgoSort.java").read_bytes()),
            ])
            manifest = discover(artifact, source, "release", REVISION, REPOSITORY)
            path = scratch / "cases.json"
            path.write_text(json.dumps(manifest))
            cases = verify_cases(load_cases(path), artifact, source, "release")
            case = next(row for row in cases.values() if row["member"] == "Lfixture/AlgoSort;->merge([I)[I")
            sites = case["verified"][0]["expected"]["dex_sites"]
            self.assertEqual(sites, [{"pc": 4, "owner": "[I", "name": "clone",
                                      "descriptor": "()Ljava/lang/Object;"}])
            display = "static int[] merge(int[] values) { return values.clone(); }"
            self.assertEqual(compare(case, {"text": display})[0]["status"], "PASS")

    def test_source_owner_is_ambiguous_across_module_source_sets(self):
        original = (FIXTURE / "InvokeBasic.java").read_bytes()
        with TemporaryDirectory() as directory:
            source = Path(directory) / "source.tar.gz"
            source_archive(source, [
                ("first/src/main/java/fixture/InvokeBasic.java", original),
                ("second/src/main/java/fixture/InvokeBasic.java", original),
                ("first/src/test/java/fixture/Ignored.java", b"ignored"),
            ])
            self.assertEqual(source_files(source, "release"), {})


if __name__ == "__main__":
    unittest.main()
