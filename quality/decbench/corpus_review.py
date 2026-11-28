"""Find source-aligned public corpus methods that merit further review."""

from collections import Counter
from bisect import bisect_left
import re

import reporting


REVIEW_KIND = "diana-decbench-review"
COMMENTS_AND_LITERALS = re.compile(
    r'/\*[\s\S]*?\*/|//[^\r\n]*|"""[\s\S]*?"""|"(?:[^"\\\r\n]|\\.)*"|\'(?:[^\'\\\r\n]|\\.)*\''
)
MARKERS = {
    "explicit_jumps": re.compile(r"\bgoto\s+\w+\s*;"),
    "resource_scopes": re.compile(r"\btry\s*\("),
    "suppressed_exception_calls": re.compile(r"\.\s*addSuppressed\s*\("),
    "exception_handlers": re.compile(r"\bcatch\s*\("),
    "explicit_effect_views": re.compile(r"\bvia\s+(?:char|byte|short|boolean)\b"),
    "unresolved_type_placeholders": re.compile(r"(?<!\?)\?\?\s+[\w$]+"),
    "nested_conditional_expression": re.compile(r"\?(?!\?)[^;?\r\n]+\?(?!\?)[^;:\r\n]+:[^;\r\n]+:"),
}
SHARED_MARKERS = {"explicit_jumps", "unresolved_type_placeholders"}


def lexical_markers(text):
    masked = COMMENTS_AND_LITERALS.sub(lambda match: re.sub(r"[^\r\n]", " ", match[0]), text)
    lines = text.split("\n")
    newlines = [match.start() for match in re.finditer("\n", masked)]
    hits = {}
    for kind, pattern in MARKERS.items():
        positions = sorted({bisect_left(newlines, match.start()) for match in pattern.finditer(masked)})
        if positions:
            hits[kind] = [{"line": index + 1, "text": lines[index].strip()} for index in positions]
    return hits


def visible_observation(method):
    observation = method.get("observation") or {}
    return {
        key: observation[key] for key in ("text", "error", "issues", "diagnostics", "has_body") if key in observation
    }


def complete_publication(observation):
    text = observation.get("text")
    return not observation.get("error") and isinstance(text, str) and bool(text.strip())


def signals_for(target, peer):
    target_text, peer_text = target.get("text"), peer.get("text")
    target_available = complete_publication(target)
    peer_available = complete_publication(peer)
    if not target_available and not peer_available:
        return []
    if not target_available or not peer_available:
        backend = "peer" if target_available else "target"
        return [{"kind": "publication_unavailable", "side": backend}]
    result = []
    for side, arm in (("target", target), ("peer", peer)):
        if arm.get("issues"):
            result.append({"kind": "recovery_diagnostics", "side": side, "issues": arm["issues"]})
    left, right = lexical_markers(target_text), lexical_markers(peer_text)
    for kind in MARKERS:
        if (kind in left) != (kind in right):
            result.append({"kind": kind, "target": left.get(kind, []), "peer": right.get(kind, [])})
        elif kind in SHARED_MARKERS and kind in left and kind in right:
            result.append({"kind": "shared_output_marker", "marker": kind, "target": left[kind], "peer": right[kind]})
    return result


def review(reports, target_backend="diana", peer_backend="jadx"):
    if target_backend == peer_backend:
        raise ValueError("the review requires two distinct backend names")
    latest = {target_backend: {}, peer_backend: {}}
    for report in reports:
        methods, _ = reporting.index_methods(report)
        backend = report["backend"]
        if backend in latest:
            for key, (_, method, _) in methods.items():
                if key[0].startswith("public:") and key[1] and key[2] and method.get("source"):
                    latest[backend][key] = (method, reporting.metadata(report))
    target, peer = latest[target_backend], latest[peer_backend]
    common = target.keys() & peer.keys()
    source_only = peer_backend is None
    findings = []
    complete_pairs = 0
    unverified_pairs = 0
    for key in sorted(target if source_only else common):
        suite, artifact, oracle, at, member = key
        left, left_meta = target[key]
        right, right_meta = peer.get(key, ({}, {}))
        a, b = visible_observation(left), visible_observation(right)
        if any(side.get("provenance", {}).get(fact) is False for side in (left, right)
               for fact in ("original_verified", "source_verified")):
            unverified_pairs += 1
            continue
        if complete_publication(a) and complete_publication(b):
            complete_pairs += 1
        if source_only:
            signals = [{"kind": kind, "target": lines}
                       for kind, lines in lexical_markers(a.get("text") or "").items()
                       if kind in SHARED_MARKERS or kind == "explicit_effect_views"]
            if a.get("issues"):
                signals.append({"kind": "recovery_diagnostics", "side": "target", "issues": a["issues"]})
        else:
            signals = signals_for(a, b)
        for side, row in (("target", left), ("peer", right)):
            body = next((check for check in row.get("checks", []) if check["metric"] == "body_recovery"), None)
            if body and body["status"] == "FAIL":
                signals.append({"kind": "body_recovery_gap", "side": side, "check": body})
        shape_a, shape_b = left.get("source_structure"), right.get("source_structure")
        if source_only and shape_a and shape_a["outcome"] == "DIFFERENT":
            signals.append({"kind": "source_cfg_difference", "target": shape_a})
        elif (shape_a and shape_b and shape_a.get("expected") == shape_b.get("expected")
                and "SETUP_ERROR" not in (shape_a["outcome"], shape_b["outcome"])):
            outcomes = shape_a["outcome"], shape_b["outcome"]
            if outcomes[0] != outcomes[1]:
                signals.append({"kind": "source_cfg_difference", "target": shape_a, "peer": shape_b})
            elif outcomes[0] == "DIFFERENT":
                signals.append({"kind": "shared_source_cfg_difference", "target": shape_a, "peer": shape_b})
        if signals:
            findings.append({"suite": suite, "input_sha256": artifact, "oracle_sha256": oracle,
                             "at": at, "member": member, "source": left["source"], "signals": signals,
                             **{field: left[field] for field in ("module", "source_forms") if field in left},
                             "target": {**left_meta, "observation": a},
                             **({} if source_only else {"peer": {**right_meta, "observation": b}})})
    return {"kind": REVIEW_KIND, "schema_version": 1,
            "target": target_backend, "peer": peer_backend,
            "coverage": ({"source_aligned": len(target) - unverified_pairs, "unverified": unverified_pairs}
                         if source_only else {"paired": len(common), "target_only": len(target.keys() - peer.keys()),
                                              "peer_only": len(peer.keys() - target.keys())}),
            "complete_publication_pairs": complete_pairs, "unverified_pairs": unverified_pairs,
            "methods": findings}


def render(document):
    reporting.require_report(document, REVIEW_KIND)
    label = reporting.label
    rows = []
    groups = {}
    for index, method in enumerate(reporting.rows(document, "methods")):
        title = f"<code>{label(method['member'])}</code> <small>{label(method['at'])}</small>"
        anchor = f"review-method-{index}"
        signals = reporting.format_data(method["signals"])
        panes = []
        for arm in ("target", "peer") if "peer" in method else ("target",):
            snapshot = method[arm]
            view = reporting.render_observation(snapshot["observation"])
            panes.append(f"<div><h4>{label(snapshot['backend'])}</h4>{view}</div>")
        rows.append(f"<details id='{anchor}'><summary>{title}</summary>{reporting.render_source(method['source'])}"
                    f"<pre>{signals}</pre><div class='publications'>{''.join(panes)}</div></details>")
        for group in {signal["kind"] for signal in method["signals"]}:
            groups.setdefault(group, []).append(f"<li><a href='#{anchor}'>{title}</a></li>")
    counts = Counter(signal["kind"] for method in document["methods"] for signal in method["signals"])
    coverage = {"methods": document["coverage"], "signals": counts}
    if document["peer"] is not None:
        coverage.update(complete_publication_pairs=document.get("complete_publication_pairs"),
                        unverified_pairs=document.get("unverified_pairs"))
    navigation = "".join(f"<details><summary>{label(group.replace('_', ' '))} · {len(items)}</summary>"
                         f"<ol>{''.join(items)}</ol></details>" for group, items in sorted(groups.items()))
    heading = label(document["target"])
    if document["peer"] is not None:
        heading += f" and {label(document['peer'])}"
    return reporting.html_page("Diana DecBench review",
                               f"<h2>{heading}</h2><pre>{reporting.format_data(coverage)}</pre>{navigation}{''.join(rows)}")
