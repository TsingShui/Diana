"""Render source-aligned findings from a public campaign for human review."""

from collections import Counter, defaultdict
from difflib import SequenceMatcher
from hashlib import sha256
import json
from pathlib import Path
from tarfile import TarError
from textwrap import dedent
from urllib.parse import urlsplit

import corpus_assets
import corpus_comparison
from corpus_models import method_source
from corpus_oracles import Unavailable, logical_path, read_sources
from corpus_review import lexical_markers
import reporting


TEMPLATE = Path(__file__).with_name("bench_view.html")
FLOW_GROUPS = {
    "分支判断": {"test", "switch"},
    "循环": {"while", "do", "for", "foreach"},
    "返回出口": {"return"},
    "提前跳转": {"break", "continue", "throw"},
}
MARKERS = {
    "explicit_jumps": "输出中有显式跳转",
    "unresolved_type_placeholders": "输出中有未确定的类型",
    "explicit_effect_views": "输出中有显式类型转换视图",
}
CHECK_LABELS = {
    "body_recovery": "方法体恢复",
    "declaration_types": "声明类型",
    "call_argument_values": "调用参数值",
    "call_argument_types": "调用参数类型",
    "field_bindings": "字段访问",
    "modeled_behavior": "执行模型",
    "corpus_quality": "调用显示",
}
SORT_ORDER = ("behavior", "recovery", "flow", "readability", "checks")


def short_member(member):
    owner, _, method = member.partition("->")
    return owner[1:-1].rsplit("/", 1)[-1] + "." + method.split("(", 1)[0]


def flow_changes(shape):
    before, after = (Counter(node["kind"] for node in shape.get(side) or [])
                     for side in ("expected", "observed"))
    return [{"label": label, "source": left, "diana": right}
            for label, kinds in FLOW_GROUPS.items()
            for left, right in [(sum(before[kind] for kind in kinds), sum(after[kind] for kind in kinds))]
            if left != right]


def check_summary(check):
    metric = check.get("metric")
    expected = check.get("expected")
    observed = check.get("observed")
    label = CHECK_LABELS.get(metric, "评测规则")
    if metric == "corpus_quality" and isinstance(expected, dict) and isinstance(observed, dict):
        old = Counter(map(tuple, expected.get("calls", [])))
        new = Counter(map(tuple, observed.get("calls", [])))
        if missing := list((old - new).elements()):
            names = "、".join(f"{name}（{arity} 个参数）" for name, arity in missing)
            return f"调用检查器未匹配到 {names}"
        if old != new:
            return "调用检查器在输出中识别到额外的调用"
        return "调用检查器识别到显示顺序差异"
    if metric == "body_recovery":
        return "原始方法有指令，Diana 只输出了声明"
    return f"{label}检查发现差异"


def findings(method):
    provenance = method.get("provenance", {})
    if not provenance.get("original_verified") or not provenance.get("source_verified"):
        return {}
    notes = defaultdict(list)
    observation = method.get("observation") or {}
    for issue in observation.get("issues", []):
        notes["recovery"].append(issue.get("reason") or "Diana 标记了局部恢复问题")
    shape = method.get("source_structure") or {}
    if shape.get("outcome") == "DIFFERENT":
        notes["flow"].append("解析出的语句控制流结构发生变化")
    markers = lexical_markers(observation.get("text") or "")
    notes["readability"].extend(MARKERS[kind] for kind in MARKERS if kind in markers)
    models = [check for check in method["checks"] if check["metric"] == "modeled_behavior"]
    failures = sum(check["status"] == "FAIL" for check in models)
    if failures:
        notes["behavior"].append(f"{failures} / {len(models)} 个执行场景失败")
    for check in method["checks"]:
        if check["status"] == "FAIL" and check["metric"] in CHECK_LABELS and check["metric"] != "modeled_behavior":
            category = "recovery" if check["metric"] == "body_recovery" else "checks"
            notes[category].append(check_summary(check))
    return {kind: list(dict.fromkeys(notes[kind])) for kind in SORT_ORDER if notes[kind]}


def code_lines(text):
    if not text:
        return []
    lines = text.splitlines()
    # Export fragments begin at the declaration token. Restore the common
    # outer indent temporarily so the two methods have the same margin.
    if len(lines) > 1 and lines[0] == lines[0].lstrip():
        margin = lines[-1][:-len(lines[-1].lstrip())] if lines[-1].lstrip() else ""
        lines[0] = margin + lines[0]
    return dedent("\n".join(lines)).splitlines()


def compare_lines(original, recovered):
    if not original or not recovered or max(len(original), len(recovered)) > 2_000:
        return [["replace", 0, len(original), 0, len(recovered)]]
    return SequenceMatcher(None, [line.strip() for line in original],
                           [line.strip() for line in recovered]).get_opcodes()


def pinned_lines(path, data):
    try:
        logical_path(path)
        return (sha256(data).hexdigest(), data.decode("utf-8").splitlines()) if data is not None else None
    except (UnicodeError, ValueError):
        return None


def source_range(pin, sources):
    pinned = sources.get(pin["path"])
    if pinned is None:
        return []
    digest, lines = pinned
    start, end = pin["start_line"], pin["end_line"]
    if digest != pin["sha256"] or not 1 <= start <= end <= len(lines):
        return []
    return code_lines("\n".join(lines[start - 1:end]))


def source_link(pin):
    link = pin.get("url", "")
    url = urlsplit(link)
    return link if url.scheme == "https" and url.hostname and url.username is None else ""


def recovery_lines(method):
    observation = method.get("observation") or {}
    text = observation.get("text") or ""
    if text and not observation.get("fragment"):
        try:
            text = method_source(text, method["member"])
        except Unavailable:
            return []
    return code_lines(text)


def snapshot(reports, projects, cache):
    latest = defaultdict(dict)
    for report in reports:
        if report["backend"] != "diana":
            continue
        for item in report["inputs"]:
            corpus = item["corpus"]["id"]
            for method in item["methods"]:
                latest[corpus][method["at"]] = method

    output = []
    for corpus, methods in latest.items():
        rows = [(method, notes) for method in methods.values() if (notes := findings(method))]
        if not rows:
            continue
        project = projects[corpus]
        paths = {logical_path(method["source"]["path"]) for method, _ in rows}
        try:
            files = read_sources(cache / corpus / "source.tar.gz", project.source_root, paths)
        except (OSError, ValueError, TarError):
            files = {}
        sources = {path: pinned_lines(path, data) for path, data in files.items()}
        for method, notes in rows:
            pin = method["source"]
            original = source_range(pin, sources)
            recovered = recovery_lines(method)
            descriptor = method["member"]
            output.append({
                "id": f"{corpus}/{method['at']}", "name": short_member(descriptor),
                "descriptor": descriptor, "project": f"{project.name} {project.version}",
                "path": pin["path"], "line": pin["start_line"], "sourceUrl": source_link(pin),
                "notes": notes, "flow": flow_changes(method.get("source_structure") or {}),
                "scenarios": [{"id": check["id"], "source": check["expected"], "diana": check["observed"]}
                              for check in method["checks"] if check["metric"] == "modeled_behavior"
                              and check["status"] == "FAIL"],
                "source": original, "diana": recovered, "diff": compare_lines(original, recovered),
            })
    output.sort(key=lambda row: (SORT_ORDER.index(next(iter(row["notes"]))), row["name"], row["id"]))
    return {"scanned": sum(map(len, latest.values())), "methods": output}


def render(reports, projects, cache):
    payload = json.dumps(snapshot(reports, projects, cache), ensure_ascii=False, separators=(",", ":"))
    # Text is only used through textContent in the browser. Keep it inside the
    # JSON script element even when either Java method contains HTML-like text.
    return TEMPLATE.read_text(encoding="utf-8").replace("<!-- BENCH_DATA -->", payload.replace("<", "\\u003c"))


def from_evidence(path, runs, cache=None):
    budget = [corpus_comparison.MAX_COMPARISON_BYTES]
    reports = []
    if path is not None:
        path, campaign, units, _ = corpus_comparison.load_campaign(path, budget)
        cache = cache or Path(campaign["prepared"])
        reports = [corpus_comparison.unit_report(path, campaign, unit, budget)[0]
                   for unit in units.values() if unit["backend"] == "diana" and unit["status"] != "SETUP_ERROR"]
    for run in runs:
        report = corpus_comparison.read_document(run, budget)
        reporting.index_methods(report)
        cache = cache or Path(report["prepared"])
        reports.append(report)
    if cache is None:
        raise ValueError("bench requires --campaign or --run")
    return render(reports, corpus_assets.load_projects(), cache)
