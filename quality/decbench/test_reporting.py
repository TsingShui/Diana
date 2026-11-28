from copy import deepcopy
import unittest

from reporting import (
    build_index, compare_reports, format_check_value, mine_reports, render_index, render_report,
)


def check(status="PASS", *, expected=7, observed=7, detail=None):
    return {
        "id": "divide", "metric": "behavior", "status": status,
        "expected": expected, "observed": observed, "detail": detail,
    }


def method(at, *checks):
    return {
        "at": at, "member": "LC;->run()I", "checks": list(checks),
        "observation": {"text": "return 7;", "argv": ["diana"], "exit_code": 0},
    }


def run(*methods, artifact="a", oracle="o", path="scratch/classes.dex"):
    return {
        "schema_version": 1, "kind": "diana-decbench-run",
        "binary": {"path": "diana", "sha256": "d"},
        "inputs": [{
            "suite": "checks", "path": path, "sha256": artifact,
            "oracle_sha256": oracle, "methods": list(methods),
        }],
    }


class ComparisonTest(unittest.TestCase):
    def test_distinguishes_original_physical_locations_and_sweeps_both_directions(self):
        baseline = run(method("j1:m0", check()), method("j2:m0", check("FAIL", observed=8)))
        candidate = run(
            method("j1:m0", check("FAIL", observed=8)), method("j2:m0", check()),
            path="other-worktree/classes.dex",
        )
        result = compare_reports(baseline, candidate)
        first, second = result["methods"]
        self.assertEqual(first["checks"][0]["transition"], "newly_failing")
        self.assertEqual(second["checks"][0]["transition"], "recovered")
        self.assertEqual(result["summary"]["newly_failing"], 1)
        self.assertEqual(result["summary"]["recovered"], 1)

    def test_changed_input_or_oracle_does_not_certify_repair(self):
        baseline = run(method("j1:m0", check("FAIL", observed=8)))
        for candidate in (
            run(method("j1:m0", check()), artifact="new"),
            run(method("j1:m0", check()), oracle="new"),
        ):
            with self.subTest(scope=candidate["inputs"][0]):
                result = compare_reports(baseline, candidate)
                self.assertEqual(result["summary"]["recovered"], 0)
                self.assertEqual(result["summary"]["coverage_added"], 1)
                self.assertEqual(result["summary"]["coverage_lost"], 1)
                self.assertEqual(len(result["input_coverage"]), 2)

    def test_loss_of_executable_evidence_does_not_certify_repair(self):
        result = compare_reports(
            run(method("j1:m0", check("FAIL", observed=8))),
            run(method("j1:m0", check("UNCHECKED", observed=None))),
        )
        self.assertEqual(result["summary"]["recovered"], 0)
        self.assertEqual(result["summary"]["coverage_lost"], 1)

    def test_changed_expectations_or_missing_digests_make_results_incomparable(self):
        results = [
            compare_reports(
                run(method("j1:m0", check())),
                run(method("j1:m0", check("FAIL", expected=8))),
            ),
            compare_reports(
                run(method("j1:m0", check()), artifact=None),
                run(method("j1:m0", check("FAIL")), artifact=None),
            ),
        ]
        for result in results:
            self.assertEqual(result["summary"]["coverage_changed"], 1)
            self.assertEqual(result["summary"]["newly_failing"], 0)

    def test_keeps_unchanged_verdict_and_changed_observation_visible(self):
        result = compare_reports(
            run(method("j1:m0", check("FAIL", observed=8))),
            run(method("j1:m0", check("FAIL", observed=9))),
        )
        verdict = result["methods"][0]["checks"][0]
        self.assertEqual(verdict["transition"], "unchanged")
        self.assertTrue(verdict["evidence_changed"])
        self.assertIn("candidate failure", render_report(result))

    def test_rejects_duplicate_methods_and_duplicate_checks(self):
        baseline = run(method("j1:m0", check()))
        with self.assertRaisesRegex(ValueError, "duplicate method"):
            compare_reports(baseline, run(method("j1:m0", check()), method("j1:m0", check())))
        with self.assertRaisesRegex(ValueError, "duplicate check"):
            compare_reports(baseline, run(method("j1:m0", check(), check())))

    def test_comparison_ignores_path_changes_but_retains_changed_publication(self):
        baseline = run(method("j1:m0", check()))
        candidate = deepcopy(baseline)
        observation = candidate["inputs"][0]["methods"][0]["observation"]
        observation["argv"][0] = "/other/worktree/diana"
        self.assertFalse(compare_reports(baseline, candidate)["methods"][0]["publication_changed"])
        observation["text"] = "return 3 + 4;"
        result = compare_reports(baseline, candidate)
        self.assertTrue(result["methods"][0]["publication_changed"])
        self.assertEqual(result["summary"]["unchanged"], 1)
        self.assertIn("publication changed", render_report(result))

    def test_html_escapes_observed_source_and_build_metadata(self):
        text = "<script>bad()</script>"
        baseline = run(method("j1:m0", check()))
        candidate = deepcopy(baseline)
        candidate["binary"]["path"] = text
        candidate["inputs"][0]["methods"][0]["observation"]["text"] = text
        candidate["inputs"][0]["methods"][0]["checks"][0] = check("FAIL", observed=text)
        for document in (candidate, compare_reports(baseline, candidate)):
            html = render_report(document)
            self.assertIn("&lt;script&gt;bad()&lt;/script&gt;", html)
            self.assertIn("href='#method-0'", html)
            self.assertNotIn(text, html)


class SnapshotsTest(unittest.TestCase):
    def test_historical_evaluator_change_keeps_peer_evidence_without_attributing_a_regression(self):
        before = run(method("j1:m0", check())) | {"evaluator_sha256": "previous"}
        after = run(method("j1:m0", check("FAIL", observed=8))) | {"evaluator_sha256": "new"}
        peer = run(method("j1:m0", check())) | {"backend": "reference", "evaluator_sha256": "peer"}
        comparison = compare_reports(before, after)
        self.assertFalse(comparison["evaluator_compatible"])
        self.assertEqual(comparison["summary"]["coverage_changed"], 1)
        self.assertEqual(comparison["summary"]["newly_failing"], 0)
        index = build_index([before, peer, after])
        finding, = index["mining"]
        self.assertEqual(finding["signals"], ["peer_gap"])
        self.assertNotIn("regression", finding["signals"])

    def test_revisit_one_corpus_without_losing_other_corpus_findings(self):
        first = run(method("j1:m0", check()), artifact="first")
        other = run(method("j1:m0", check("FAIL", observed=9)), artifact="other")
        peer = first | {"backend": "peer"}
        regression = run(method("j1:m0", check("FAIL", observed=8)), artifact="first")
        index = build_index([first, other, peer, regression])
        rows = {case["input_sha256"]: case for case in index["mining"]}
        self.assertEqual(set(rows), {"first", "other"})
        self.assertEqual(rows["first"]["signals"], ["regression", "peer_gap"])
        self.assertEqual(rows["other"]["unmeasured"], ["peer"])
        self.assertEqual([(item["baseline"], item["candidate"]) for item in index["history"]], [(0, 3)])
        self.assertIn("no matching method", render_index(index))

    def test_narrow_rerun_updates_only_the_selected_original_method(self):
        failed = run(method("j1:m0", check("FAIL", observed=8)), method("j1:m1", check("FAIL", observed=9)))
        selected = run(method("j1:m0", check()))
        findings = mine_reports([failed, selected])
        self.assertEqual([case["at"] for case in findings], ["j1:m1"])
        self.assertEqual(findings[0]["arms"][0]["run"], 0)
        self.assertEqual(build_index([failed, selected])["history"], [])

    def test_current_oracle_replaces_only_the_matching_profile(self):
        first = run(method("j1:m0", check("FAIL", observed=8)))
        correction = run(method("j1:m0", check()), oracle="replacement")
        self.assertEqual(mine_reports([first, correction]), [])
        first["inputs"][0]["corpus"] = {"profile": "modeled"}
        correction["inputs"][0]["corpus"] = {"profile": "reviewed"}
        self.assertEqual([case["profile"] for case in mine_reports([first, correction])], ["modeled"])

    def test_mines_a_regression_that_a_peer_already_passes(self):
        first = run(method("j1:m0", check()))
        peer = run(method("j1:m0", check())) | {"backend": "reference", "link": "reference.html"}
        current = run(method("j1:m0", check("FAIL", observed=8))) | {"revision": "current"}
        index = build_index([first, peer, current])
        self.assertEqual(len(index["history"]), 1)
        self.assertEqual(index["history"][0]["candidate"], 2)
        self.assertEqual(index["history"][0]["summary"]["newly_failing"], 1)
        self.assertEqual(len(index["mining"]), 1)
        case = index["mining"][0]
        self.assertEqual(case["regressions"], ["diana"])
        self.assertEqual(case["passing"], ["reference"])
        self.assertEqual(case["signals"], ["regression", "peer_gap"])
        self.assertIn("href='reference.html'", render_index(index))

    def test_only_equal_oracles_and_expectations_provide_peer_evidence(self):
        failing = run(method("j1:m0", check("FAIL", observed=8)))
        reports = [
            failing,
            run(method("j1:m0", check("FAIL", observed=8))) | {"backend": "also-fails"},
            run(method("j1:m0", check()), oracle="other") | {"backend": "different-oracle"},
            run(method("j1:m0", check()), artifact="other") | {"backend": "different-artifact"},
            run(method("j1:m0", check(expected=8))) | {"backend": "different-expectation"},
            run(method("j2:m0", check())) | {"backend": "different-locator"},
            run(method("j1:m0", check("UNCHECKED", observed=None))) | {"backend": "unchecked"},
        ]
        mined = mine_reports(reports)
        self.assertEqual(len(mined), 1)
        case = mined[0]
        self.assertEqual(case["signals"], ["all_measured_failing"])
        self.assertEqual(case["failing"], ["also-fails", "diana"])
        self.assertFalse(case["passing"])
        self.assertEqual(case["unmeasured"], [
            "different-artifact", "different-expectation", "different-locator",
            "different-oracle", "unchecked",
        ])

    def test_mining_keeps_only_latest_observation_per_backend(self):
        failed = run(method("j1:m0", check("FAIL", observed=8)))
        repaired = run(method("j1:m0", check()))
        self.assertEqual(mine_reports([failed, repaired]), [])
        index = build_index([failed, repaired])
        self.assertEqual(index["history"][0]["summary"]["recovered"], 1)

    def test_snapshot_links_do_not_allow_external_or_script_targets(self):
        snapshot = run(method("j1:m0", check()))
        for link in ("https://example.com", "javascript:bad()", "//example.com", r"\example.com"):
            with self.subTest(link=link):
                html = render_index(build_index([snapshot | {"link": link, "revision": "<script>"}]))
                self.assertNotIn("href=", html)
                self.assertIn("&lt;script&gt;", html)


if __name__ == "__main__":
    unittest.main()


class StructurePresentationTest(unittest.TestCase):
    def test_graph_edges_are_compact_and_escaped(self):
        graph = [
            {"kind": "entry", "edges": [["next", 1]]},
            {"kind": "<switch>", "edges": [["case:<x>", 0]]},
        ]
        text = format_check_value("structure", graph)
        self.assertIn("next → 1", text)
        self.assertIn("&lt;switch&gt;", text)
        self.assertIn("case:&lt;x&gt; → 0", text)
        self.assertNotIn("<switch>", text)
