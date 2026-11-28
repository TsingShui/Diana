"""Review only paired method identities and keep observations separate from judgments."""

import unittest

import corpus_review
import reporting


AT = "e2:l0:c1:m3"
MEMBER = "Lsample/Example;->update()V"
SOURCE = {"path": "src/Example.java", "start_line": 2, "end_line": 7,
          "url": "https://example.org/revision/Example.java#L2-L7", "text": "/* <source> */"}


def snapshot(backend, text, *, artifact="input", oracle="oracle", error=None, at=AT):
    observation = {"text": text}
    if error:
        observation["error"] = error
    return {
        "schema_version": 1, "kind": reporting.RUN_KIND, "backend": backend,
        "inputs": [{"suite": "public:example", "sha256": artifact, "oracle_sha256": oracle,
                    "methods": [{"at": at, "member": MEMBER, "checks": [], "source": SOURCE,
                                 "observation": observation}]}],
    }


class PublicReviewTest(unittest.TestCase):
    def test_marks_cleanup_presentation_without_a_quality_verdict(self):
        diana = snapshot("diana", "void update() {\n  body.addSuppressed(close);\n}")
        jadx = snapshot("jadx", "void update() {\n  try (Input in = open()) {}\n}")
        result = corpus_review.review([diana, jadx])
        self.assertEqual(result["coverage"], {"paired": 1, "target_only": 0, "peer_only": 0})
        self.assertEqual(result["complete_publication_pairs"], 1)
        method, = result["methods"]
        self.assertEqual([signal["kind"] for signal in method["signals"]],
                         ["resource_scopes", "suppressed_exception_calls"])
        self.assertEqual(method["signals"][1]["target"],
                         [{"line": 2, "text": "body.addSuppressed(close);"}])
        self.assertNotIn("status", method)
        html = reporting.render_report(result)
        self.assertIn("https://example.org/revision/Example.java#L2-L7", html)
        self.assertIn("&lt;source&gt;", html)

    def test_input_oracle_and_coordinates_must_match(self):
        target = snapshot("diana", "goto next;")
        for peer in (snapshot("jadx", "return;", artifact="another input"),
                     snapshot("jadx", "return;", oracle="another oracle"),
                     snapshot("jadx", "return;", at="e2:l0:c1:m2")):
            result = corpus_review.review([target, peer])
            self.assertEqual(result["coverage"], {"paired": 0, "target_only": 1, "peer_only": 1})
            self.assertEqual(result["methods"], [])

    def test_latest_matching_method_hides_stale_signals_and_retains_other_scopes(self):
        older = snapshot("diana", "goto next;")
        newer = snapshot("diana", "return;")
        unrelated = snapshot("diana", "goto stale;", artifact="unmatched")
        peer = snapshot("jadx", "return;")
        result = corpus_review.review([older, newer, unrelated, peer])
        self.assertEqual(result["coverage"], {"paired": 1, "target_only": 1, "peer_only": 0})
        self.assertEqual(result["methods"], [])

    def test_comments_strings_and_text_blocks_cannot_create_signals(self):
        text = '''/* goto next; */
            var a = "try (resource)";
            var b = """
                exception.addSuppressed(other);
                catch (Exception expected)
                """;
            // temporary via short
            return;'''
        self.assertEqual(corpus_review.lexical_markers(text), {})

    def test_peer_can_supply_recoverable_body_when_target_is_partial(self):
        diana = snapshot("diana", "goto next;", error="partial")
        jadx = snapshot("jadx", "return;")
        method, = corpus_review.review([diana, jadx])["methods"]
        self.assertEqual(method["signals"], [{"kind": "publication_unavailable", "side": "target"}])

    def test_shared_output_and_cfg_differences_remain_review_signals(self):
        diana = snapshot("diana", "void update() { goto join; }")
        jadx = snapshot("jadx", "void update() { goto fallback; }")
        for item in (diana, jadx):
            row, = item["inputs"][0]["methods"]
            row["source_structure"] = {"outcome": "DIFFERENT", "expected": [{"kind": "return"}]}
        method, = corpus_review.review([diana, jadx])["methods"]
        self.assertEqual([item["kind"] for item in method["signals"]],
                         ["shared_output_marker", "shared_source_cfg_difference"])
        self.assertEqual(method["signals"][0]["marker"], "explicit_jumps")
        self.assertNotIn("status", method)

    def test_excludes_unverified_comparison_and_reports_identified_partial_pair(self):
        diana = snapshot("diana", "goto next;", error="partial")
        jadx = snapshot("jadx", "return;")
        result = corpus_review.review([diana, jadx])
        self.assertEqual(result["coverage"]["paired"], 1)
        self.assertEqual(result["complete_publication_pairs"], 0)
        diana["inputs"][0]["methods"][0]["provenance"] = {"source_verified": False}
        result = corpus_review.review([diana, jadx])
        self.assertEqual(result["coverage"]["paired"], 1)
        self.assertEqual(result["unverified_pairs"], 1)
        self.assertEqual(result["methods"], [])
