"""Check campaign selection, independent arms, and durable evidence."""

from contextlib import closing
import json
from pathlib import Path
from subprocess import TimeoutExpired
from tempfile import TemporaryDirectory
from time import perf_counter
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import candidates
import corpus_campaign
import evidence
import judge


def read(path):
    return json.loads(path.read_text(encoding="utf-8"))


def method(owner, number):
    return {"at": f"e0:l0:c0:m{number}", "member": f"L{owner};->example{number}()V",
            "source": {"path": f"{owner}.java", "start_line": 1, "end_line": 3}}


class CampaignTest(unittest.TestCase):
    def setUp(self):
        self.root = Path(self.enterContext(TemporaryDirectory()))
        self.state = self.root / "state"
        self.output = self.root / "runs"
        self.cache = self.root / "cache"
        self.diana = self.root / "diana"
        self.jadx = self.root / "jadx.jar"
        self.diana.write_bytes(b"native producer")
        self.jadx.write_bytes(b"peer producer")
        self.projects, self.reviewed = {}, {}
        for identifier in ("alpha", "beta"):
            manifest = self.root / f"{identifier}.cases.json"
            self.projects[identifier] = SimpleNamespace(
                id=identifier, manifest=manifest, source_root=f"{identifier}-root",
                source_commit="a" * 40, provenance={"source_repository_url": "https://example.org/project"},
            )
            self.reviewed[identifier] = {
                "input_sha256": evidence.digest(identifier), "revision_entry": "revision.txt",
                "methods": [method(identifier, number) for number in range(3)],
            }
            evidence.write_json(manifest, self.reviewed[identifier])
        self.enterContext(patch("corpus_campaign.corpus_assets.load_projects", return_value=self.projects))
        self.prepare = self.enterContext(patch("corpus_campaign.corpus_assets.prepare_assets", side_effect=self.assets))
        self.enterContext(patch("corpus_campaign.corpus_eval.case_configuration", side_effect=self.configuration))
        self.evaluate = self.enterContext(patch("corpus_campaign.corpus_eval.run", side_effect=self.report))
        self.coverage = self.enterContext(patch("corpus_campaign.corpus_coverage.measure", return_value={
            "schema_version": 1, "kind": "diana-decbench-coverage", "original": {"complete": True},
            "source": {"complete": True}, "selection": {"manifests": []},
            "evidence": {"unique_methods_by_tool_and_evaluator": [], "snapshots": []}, "scope": {},
        }))

    def assets(self, project, cache):
        return SimpleNamespace(apk=cache / project.id / "input.apk", source=cache / project.id / "source.tar.gz")

    def configuration(self, project, case_path):
        path = case_path or project.manifest
        return path, read(path)

    def report(self, project, cache, binary, revision, backend, selected, jobs, timeout, *, case_path=None, **options):
        _, config = self.configuration(project, case_path)
        status = "FAIL" if backend == "diana" else "PASS"
        observed = []
        for case in config["methods"]:
            if case["at"] in selected:
                observed.append({**case, "status": status, "checks": [{
                    "id": "selected-call", "metric": "corpus_quality", "status": status,
                    "expected": "call", "observed": "missing" if status == "FAIL" else "call",
                }], "observation": {"text": "goto Next;" if status == "FAIL" else "next();"}})
        report = judge.run_header(binary, evidence.file_digest(binary), cache, revision, backend, "evaluator")
        report["inputs"] = [{"suite": f"public:{project.id}", "path": str(self.assets(project, cache).apk),
                             "sha256": config["input_sha256"], "oracle_sha256": evidence.digest(config),
                             "methods": observed, "inventory": {"complete": True},
                             "corpus": {"profile": "discovered" if case_path else "reviewed"}}]
        return judge.summarize(report, perf_counter())

    def campaign(self, **options):
        return corpus_campaign.run(None, self.cache, self.output, self.state, self.diana,
                                   jadx_jar=self.jadx, **options)

    def test_reviewed_campaign_keeps_all_projects_and_shards_comparable(self):
        path, code = self.campaign(shards=2, revision="tested", execute_models=True, inspect_structure=True)
        result = read(path)
        self.assertEqual(code, 1)
        self.assertEqual(result["projects"], ["alpha", "beta"])
        self.assertEqual(result["checks_by_backend"], {"diana": {"FAIL": 6}, "jadx": {"PASS": 6}})
        self.assertEqual(len(result["units"]), 8)
        for project in self.projects:
            observed = [unit for unit in result["units"] if unit["corpus"] == project]
            for shard, numbers in ((1, (0, 2)), (2, (1,))):
                arms = [unit for unit in observed if unit["shard"] == shard]
                self.assertEqual({unit["backend"] for unit in arms}, {"diana", "jadx"})
                for arm in arms:
                    self.assertEqual(arm["selected"], [method(project, number)["at"] for number in numbers])
                    self.assertTrue((path.parent / arm["report"]).is_file())
                    self.assertTrue((path.parent / arm["html"]).is_file())
        self.assertTrue(all(call.kwargs["execute_models"] and call.kwargs["inspect_structure"]
                            for call in self.evaluate.call_args_list))
        self.assertTrue(all(not call.kwargs for call in self.prepare.call_args_list))

        opportunities = read(path.parent / "opportunities.json")
        self.assertEqual(len(opportunities), 6)
        self.assertTrue(all(item["signals"] == ["peer_gap"] for item in opportunities))
        self.assertEqual(read(path.parent / "review.json")["coverage"]["paired"], 6)
        self.assertEqual(len(read(path.parent / "index.json")["runs"]), 8)
        with closing(candidates.connect(self.state)) as db:
            pending = candidates.cases(db, "pending", None)
            self.assertEqual(len(pending), 6)
            self.assertTrue(all(candidates.show(self.state, item)["checks"] for item in pending))

    def test_bad_cached_project_and_backend_timeout_leave_other_units_available(self):
        def prepare(project, cache):
            if project.id == "alpha":
                raise ValueError("the cached APK is missing")
            return self.assets(project, cache)

        def evaluate(*args, **kwargs):
            if args[4] == "jadx" and args[5] == [method("beta", 1)["at"]]:
                raise TimeoutExpired(["jadx"], 1)
            return self.report(*args, **kwargs)

        self.prepare.side_effect = prepare
        self.evaluate.side_effect = evaluate
        path, code = self.campaign(shards=2)
        result = read(path)
        self.assertEqual(code, 2)
        self.assertEqual(len(result["units"]), 5)
        failed = [unit for unit in result["units"] if unit["status"] == "SETUP_ERROR"]
        self.assertEqual([(unit["corpus"], unit["stage"]) for unit in failed],
                         [("alpha", "prepare"), ("beta", "evaluation")])
        self.assertIn("TimeoutExpired", failed[1]["detail"])
        self.assertEqual(len(read(path.parent / "index.json")["runs"]), 3)
        self.assertEqual(read(path.parent / "review.json")["coverage"], {"paired": 2, "target_only": 1, "peer_only": 0})

    def test_sample_shards_share_manifests_and_keep_checkpoint_paths_stable(self):
        def prepare(artifact, source, root, revision, repository, **options):
            self.assertEqual(options["revision_entry"], "revision.txt")
            self.assertEqual(len(options["exclude"]), 3)

            def select(limit, shard):
                self.assertEqual(limit, 7)
                number, count = shard
                self.assertEqual(count, 2)
                return {"input_sha256": options["input_sha256"], "discovery": {"shard": number},
                        "methods": [method(root, number + 10)]}

            return SimpleNamespace(select=select)

        with patch("corpus_campaign.corpus_discover.prepare_discovery", side_effect=prepare) as sample:
            first, _ = self.campaign(profile="sample", limit=7, shards=2, resume=True)
            second, _ = self.campaign(profile="sample", limit=7, shards=2, resume=True)
        self.assertNotEqual(first, second)
        self.assertEqual(sample.call_count, 4)
        initial = self.evaluate.call_args_list[:8]
        repeat = self.evaluate.call_args_list[8:]
        for index, (before, after) in enumerate(zip(initial, repeat)):
            path = before.kwargs["case_path"]
            self.assertEqual(path, after.kwargs["case_path"])
            self.assertEqual(before.kwargs["checkpoint"], self.state / "checkpoints")
            self.assertEqual(path, initial[index ^ 1].kwargs["case_path"])
            self.assertEqual(before.args[5], initial[index ^ 1].args[5])
        self.assertEqual(len(list(second.parent.rglob("cases.json"))), 4)
        self.assertFalse(list(second.parent.rglob("*.apk")))
        self.assertFalse(list(second.parent.rglob("*.tar.gz")))

    def test_incomplete_original_population_preserves_valid_arms_and_returns_setup_status(self):
        available = self.coverage.return_value
        unavailable = {**available, "original": {"complete": False}}
        self.coverage.side_effect = (unavailable, available)
        path, code = self.campaign()
        result = read(path)
        self.assertEqual(code, 2)
        self.assertFalse(result["coverage"]["alpha"]["complete"])
        self.assertTrue(result["coverage"]["beta"]["complete"])
        self.assertEqual([(unit["corpus"], unit["stage"]) for unit in result["units"] if unit["status"] == "SETUP_ERROR"],
                         [("alpha", "coverage")])
        self.assertEqual(read(path.parent / "review.json")["coverage"]["paired"], 6)


if __name__ == "__main__":
    unittest.main()
