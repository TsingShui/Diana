"""Compare DecBench observations and render portable HTML evidence."""

from collections import Counter, defaultdict
from html import escape
from itertools import groupby
import json
from urllib.parse import urlsplit


RUN_KIND = "diana-decbench-run"
COMPARISON_KIND = "diana-decbench-comparison"
INDEX_KIND = "diana-decbench-index"
STATUSES = ("PASS", "FAIL", "UNCHECKED", "SETUP_ERROR")
TRANSITIONS = (
    "newly_failing", "recovered", "coverage_added", "coverage_lost",
    "coverage_changed", "unchanged",
)


def require_report(report, kind):
    if not isinstance(report, dict) or report.get("kind") != kind or report.get("schema_version") != 1:
        raise ValueError(f"expected schema 1 {kind} report")


def rows(value, key):
    if not isinstance(value, dict) or not isinstance(value.get(key), list):
        raise ValueError(f"report field {key} must be an array")
    for row in value[key]:
        if not isinstance(row, dict):
            raise ValueError(f"report field {key} must contain objects")
        yield row


def string(value, key, *, optional=False):
    field = value.get(key)
    if field is None and optional:
        return ""
    if not isinstance(field, str) or not field:
        raise ValueError(f"report field {key} must be a nonempty string")
    return field


def index_checks(method):
    indexed = {}
    for check in rows(method, "checks"):
        key = (string(check, "metric"), string(check, "id"))
        if key in indexed:
            raise ValueError(f"duplicate check {key} for {method.get('member')}")
        if check.get("status") not in STATUSES or "expected" not in check or "observed" not in check:
            raise ValueError(f"invalid observation for check {key}")
        indexed[key] = check
    return indexed


def index_methods(report):
    require_report(report, RUN_KIND)
    indexed = {}
    inputs = set()
    for artifact in rows(report, "inputs"):
        scope = (
            string(artifact, "suite"), string(artifact, "sha256", optional=True),
            string(artifact, "oracle_sha256", optional=True),
        )
        if scope in inputs:
            raise ValueError(f"duplicate input and oracle for suite {scope[0]}")
        inputs.add(scope)
        for method in rows(artifact, "methods"):
            key = (*scope, string(method, "at"), string(method, "member"))
            if key in indexed:
                raise ValueError(f"duplicate method identity {key}")
            indexed[key] = (artifact.get("path"), method, index_checks(method))
    return indexed, inputs


def transition(before, after, same_evidence):
    if before is None:
        return "coverage_added"
    if after is None:
        return "coverage_lost"
    if not same_evidence or json_identity(before["expected"]) != json_identity(after["expected"]):
        return "coverage_changed"
    old, new = before["status"], after["status"]
    if old == new:
        return "unchanged"
    if old == "PASS" and new == "FAIL":
        return "newly_failing"
    if old == "FAIL" and new == "PASS":
        return "recovered"
    if old in ("PASS", "FAIL"):
        return "coverage_lost"
    if new in ("PASS", "FAIL"):
        return "coverage_added"
    return "coverage_changed"


def metadata(report):
    return {key: report[key] for key in ("backend", "revision", "binary", "created_at", "evaluator_sha256")
            if key in report}


def publication_fields(method):
    if method is None:
        return None
    observation = method.get("observation") or {}
    return {key: observation[key] for key in ("exit_code", "text", "issues", "error") if key in observation}


def evaluator_compatible(baseline, candidate):
    return (baseline.get("backend") != candidate.get("backend")
            or baseline.get("evaluator_sha256") == candidate.get("evaluator_sha256"))


def compare_reports(baseline: dict, candidate: dict) -> dict:
    """Compare compatible independent checks, retaining any incomparable output."""
    before, old_inputs = index_methods(baseline)
    after, new_inputs = index_methods(candidate)
    compatible = evaluator_compatible(baseline, candidate)
    summary = dict.fromkeys(TRANSITIONS, 0)
    metrics = defaultdict(Counter)
    methods = []
    for key in sorted(before.keys() | after.keys()):
        suite, artifact, oracle, at, member = key
        old_path, old_method, old_checks = before.get(key, (None, None, {}))
        new_path, new_method, new_checks = after.get(key, (None, None, {}))
        checks = []
        for metric, check_id in sorted(old_checks.keys() | new_checks.keys()):
            previous = old_checks.get((metric, check_id))
            current = new_checks.get((metric, check_id))
            change = transition(previous, current, bool(artifact and oracle and compatible))
            summary[change] += 1
            metrics[metric][change] += 1
            checks.append({
                "id": check_id, "metric": metric, "transition": change,
                "baseline": previous, "candidate": current,
                "evidence_changed": previous != current,
            })
        methods.append({
            "suite": suite, "input_sha256": artifact or None, "oracle_sha256": oracle or None,
            "at": at, "member": member, "baseline_path": old_path, "candidate_path": new_path,
            "baseline": old_method, "candidate": new_method, "checks": checks,
            "publication_changed": (
                old_method is not None and new_method is not None
                and publication_fields(old_method) != publication_fields(new_method)
            ),
        })
    return {
        "schema_version": 1, "kind": COMPARISON_KIND,
        "baseline": metadata(baseline), "candidate": metadata(candidate), "summary": summary,
        "evaluator_compatible": compatible,
        "metrics": dict(sorted(metrics.items())),
        "input_coverage": [
            {
                "suite": suite, "input_sha256": artifact or None, "oracle_sha256": oracle or None,
                "baseline": (suite, artifact, oracle) in old_inputs,
                "candidate": (suite, artifact, oracle) in new_inputs,
            }
            for suite, artifact, oracle in sorted(old_inputs | new_inputs)
        ],
        "methods": methods,
    }


def format_data(value):
    return escape(json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True))


def label(value):
    return escape(str(value))


def render_source(source):
    if not source:
        return ""
    path = label(source["path"])
    lines = f"{label(source['start_line'])}–{label(source['end_line'])}"
    url = source.get("url", "")
    reference = f"<code>{path}</code>"
    if isinstance(url, str):
        try:
            parsed = urlsplit(url)
            if parsed.scheme == "https" and parsed.hostname and parsed.username is None:
                reference = f"<a href='{escape(url, quote=True)}'>{reference}</a>"
        except ValueError:
            pass
    snippet = source.get("text")
    excerpt = f"<pre>{label(snippet)}</pre>" if snippet else ""
    return f"<details><summary>Original source · {reference} · lines {lines}</summary>{excerpt}</details>"


def format_check_value(metric, value):
    if metric != "structure" or not isinstance(value, list) or not value:
        return format_data(value)
    lines = []
    for index, node in enumerate(value):
        if not isinstance(node, dict) or not isinstance(node.get("kind"), str):
            return format_data(value)
        edges = node.get("edges")
        if not isinstance(edges, list) or any(
            not isinstance(edge, list) or len(edge) != 2
            or not isinstance(edge[0], str) or type(edge[1]) != int
            for edge in edges
        ):
            return format_data(value)
        targets = ", ".join(f"{role} → {target}" for role, target in edges)
        lines.append(f"{index:>3}  {node['kind']}   {targets}".rstrip())
    return label("\n".join(lines))


def render_observation(observation):
    observation = observation or {}
    fields = {key: value for key, value in observation.items() if key != "text"}
    text = label(observation.get("text", ""))
    return f"<pre>{format_data(fields)}</pre><pre>{text}</pre>"


def render_check_rows(checks, comparison):
    parts = []
    for check in checks:
        name = f"<code>{label(check['metric'])} / {label(check['id'])}</code>"
        if comparison:
            kind = check["transition"]
            sides = [check["baseline"], check["candidate"]]
        else:
            kind = check["status"]
            sides = [check]
        cells = []
        for side in sides:
            if side is None:
                cells.append("<td>Absent</td>")
                continue
            status = label(side["status"])
            expected = format_check_value(check["metric"], side["expected"])
            observed = format_check_value(check["metric"], side["observed"])
            detail = label(side.get("detail", ""))
            original_dex = side.get("original_dex")
            raw = ""
            if original_dex:
                raw = ("<details><summary>Original DEX model · " + label(original_dex["status"])
                       + "</summary><pre>" + format_data(original_dex) + "</pre></details>")
            cells.append(
                f"<td><strong>{status}</strong><dl><dt>Expected</dt><dd><pre>{expected}</pre></dd>"
                f"<dt>Observed</dt><dd><pre>{observed}</pre></dd></dl><p>{detail}</p>{raw}</td>"
            )
        changed = (
            " · evidence changed" if comparison and check.get("evidence_changed")
            and kind == "unchanged" else ""
        )
        parts.append(
            f"<tr><th scope='row'>{name}</th><td>{label(kind + changed)}</td>"
            + "".join(cells) + "</tr>"
        )
    sides = "<th>Baseline</th><th>Candidate</th>" if comparison else "<th>Evidence</th>"
    return (
        "<table><thead><tr><th>Check</th><th>Outcome</th>" + sides
        + "</tr></thead><tbody>" + "".join(parts) + "</tbody></table>"
    )


def render_method(method, comparison, anchor):
    title = f"<code>{label(method['member'])}</code> <small>{label(method['at'])}</small>"
    if comparison:
        sides = [("Baseline", method["baseline"]), ("Candidate", method["candidate"])]
        location = (
            f"<p>{label(method['suite'])} · input <code>{label(method['input_sha256'])}</code>"
            f" · oracle <code>{label(method['oracle_sha256'])}</code></p>"
        )
        open_method = method.get("publication_changed") or any(
            check["transition"] != "unchanged" for check in method["checks"]
        )
    else:
        sides = [("Recovered publication", method)]
        location = ""
        open_method = any(check["status"] != "PASS" for check in method["checks"])
    output = []
    for name, side in sides:
        if side is None:
            output.append(f"<div><h4>{name}</h4><p>Method absent</p></div>")
        else:
            observation = render_observation(side.get("observation"))
            structure = render_source_structure(side.get("source_structure"))
            output.append(f"<details><summary>{name}</summary>{observation}{structure}</details>")
    open_attribute = " open" if open_method else ""
    source = next((side["source"] for _, side in sides if side and side.get("source")), None)
    return (
        f"<details id='{anchor}'{open_attribute}><summary>{title}</summary>{location}"
        + render_source(source)
        + render_check_rows(method["checks"], comparison)
        + "<div class='publications'>" + "".join(output) + "</div></details>"
    )


def render_source_structure(receipt):
    if not receipt:
        return ""
    graphs = "".join(f"<dt>{title}</dt><dd><pre>{format_check_value('structure', receipt[key])}</pre></dd>"
                     for key, title in (("expected", "Original source"), ("observed", "Recovered publication")))
    detail = f"<p>{label(receipt['detail'])}</p>" if receipt.get("detail") else ""
    return f"<details><summary>Source CFG · {label(receipt['outcome'])}</summary>{detail}<dl>{graphs}</dl></details>"


def method_index(methods, comparison):
    candidates = []
    for index, method in enumerate(methods):
        current = method["candidate"] if comparison else method
        failures = current and any(check["status"] == "FAIL" for check in current["checks"])
        if comparison:
            changes = {
                check["transition"] for check in method["checks"]
                if check["transition"] != "unchanged"
            }
            if method.get("publication_changed"):
                changes.add("publication changed")
            if any(check.get("evidence_changed") for check in method["checks"]):
                changes.add("check evidence changed")
        else:
            changes = {check["status"] for check in method["checks"] if check["status"] != "PASS"}
        if failures:
            changes.add("candidate failure" if comparison else "FAIL")
        if changes:
            description = label(", ".join(sorted(changes)))
            member = label(method["member"])
            at = label(method["at"])
            candidates.append(f"<li><a href='#method-{index}'><code>{member}</code> {at}</a> · {description}</li>")
    if not candidates:
        return ""
    return "<details open><summary>Methods to inspect</summary><ol>" + "".join(candidates) + "</ol></details>"


def render_report(report: dict) -> str:
    """Return a standalone HTML page with escaped reports and method evidence."""
    if isinstance(report, dict) and report.get("kind") == "diana-decbench-coverage":
        import corpus_coverage

        return corpus_coverage.render(report)
    if isinstance(report, dict) and report.get("kind") == "diana-decbench-campaign-comparison":
        import corpus_comparison

        return corpus_comparison.render(report)
    if isinstance(report, dict) and report.get("kind") == "diana-decbench-corpus-triage":
        import corpus_triage

        return corpus_triage.render(report)
    if isinstance(report, dict) and report.get("kind") == "diana-decbench-review":
        import corpus_review

        return corpus_review.render(report)
    if isinstance(report, dict) and report.get("kind") == "diana-decbench-timing":
        return render_timing(report)
    if isinstance(report, dict) and report.get("kind") == COMPARISON_KIND:
        require_report(report, COMPARISON_KIND)
        title = "Diana DecBench comparison"
        provenance = {"baseline": report["baseline"], "candidate": report["candidate"]}
        summary = report["summary"]
        coverage = f"<h2>Input coverage</h2><pre>{format_data(report['input_coverage'])}</pre>"
        columns = TRANSITIONS
        sections = "".join(
            render_method(method, True, f"method-{index}")
            for index, method in enumerate(report["methods"])
        )
        method_links = method_index(report["methods"], True)
    else:
        index_methods(report)
        title = "Diana DecBench assessment"
        provenance = metadata(report)
        summary = Counter(
            check["status"]
            for artifact in report["inputs"]
            for method in artifact["methods"]
            for check in method["checks"]
        )
        coverage = ""
        columns = STATUSES
        sections = []
        all_methods = []
        for artifact in report["inputs"]:
            heading = label(artifact["suite"])
            source = label(artifact.get("path"))
            identity = format_data({
                "input_sha256": artifact.get("sha256"), "oracle_sha256": artifact.get("oracle_sha256"),
                **({"inventory": artifact["inventory"], "corpus": artifact["corpus"]}
                   if artifact.get("corpus") else {}),
            })
            start = len(all_methods)
            methods = "".join(
                render_method(method, False, f"method-{index}")
                for index, method in enumerate(artifact["methods"], start=start)
            )
            all_methods.extend(artifact["methods"])
            sections.append(f"<section><h3>{heading}</h3><p>{source}</p><pre>{identity}</pre>{methods}</section>")
        sections = "".join(sections)
        method_links = method_index(all_methods, False)
    numbers = "".join(f"<li><strong>{label(count)}</strong> {label(name)}</li>" for name, count in summary.items())
    metrics = render_metrics(report.get("metrics", {}), columns)
    body = (
        f"<ul>{numbers}</ul>"
        f"<details><summary>Run provenance</summary><pre>{format_data(provenance)}</pre></details>"
        f"{metrics}{coverage}{method_links}<h2>Methods</h2>{sections}"
    )
    return html_page(title, body)


def render_metrics(metrics, columns):
    if not metrics:
        return ""
    show_scope = columns == STATUSES
    scope = ("fully_measured", "partially_measured", "unmeasured") if show_scope else ()
    headings = "".join(f"<th>{label(name.replace('_', ' '))}</th>" for name in (*columns, *scope))
    rows = []
    for metric, data in metrics.items():
        counts = data["checks"] if show_scope else data
        amounts = [counts.get(name, 0) for name in columns]
        amounts.extend(data["method_rows"].get(name, 0) for name in scope)
        cells = "".join(f"<td>{label(count)}</td>" for count in amounts)
        rows.append(f"<tr><th scope='row'>{label(metric)}</th>{cells}</tr>")
    return ("<h2>Evidence by metric</h2><table><thead>"
            f"<tr><th scope='col'>Metric</th>{headings}</tr></thead><tbody>{''.join(rows)}</tbody></table>")


def render_timing(report):
    require_report(report, "diana-decbench-timing")
    comparison = report["comparison"]

    def row(name, result):
        if result is None:
            return f"<tr><th scope='row'>{name}</th><td colspan='4'>Unavailable</td></tr>"
        numbers = [result[arm]["median_seconds"] * 1000 for arm in ("baseline", "candidate")]
        numbers.append(result["median_paired_delta_seconds"] * 1000)
        cells = "".join(f"<td>{value:.3f}</td>" for value in numbers)
        deviation = result.get("paired_deltas", {}).get("median_absolute_deviation_seconds")
        cells += f"<td>{deviation * 1000:.3f}</td>" if deviation is not None else "<td>—</td>"
        return f"<tr><th scope='row'>{name}</th>{cells}</tr>"

    rows = [row("Original method inventory", comparison["inventory"]),
            row("Selected retrieval total per trial", comparison["method_totals"])]
    for method in comparison["methods"]:
        name = f"<code>{label(method['member'])}</code> <small>{label(method['at'])}</small>"
        publication = method.get("publication")
        if publication and not publication["identical_all_trials"]:
            name += f"<pre>{format_data(publication['distinct_outputs'])}</pre>"
        rows.append(row(name, method["comparison"]))
    provenance = format_data(report["provenance"])
    status = "complete" if report["comparable"] else "incomplete"
    return html_page("Diana DecBench paired timings", (
        f"<p>{len(report['measurements'])} paired trials. Evidence {status}.</p>"
        f"<details><summary>Input and tool provenance</summary><pre>{provenance}</pre></details>"
        f"<div><h2>Paired CLI latency in milliseconds</h2><table><thead><tr>"
        "<th>Operation</th><th>Baseline median</th><th>Candidate median</th>"
        f"<th>Median paired difference</th><th>Median absolute deviation</th></tr></thead>"
        f"<tbody>{''.join(rows)}</tbody></table></div>"
        f"<pre>{format_data(report['errors'])}</pre>"
    ))


def html_page(title, body):
    return f"""<!doctype html>
<html lang="en">
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'">
<title>{title}</title>
<style>
:root {{color-scheme: light dark;font-family:system-ui,sans-serif}}
body {{margin:0;padding:2rem;max-width:96rem}}
ul {{display:flex;gap:.65rem 1.4rem;list-style:none;padding:0;flex-wrap:wrap}}
section,details {{border:1px solid color-mix(in srgb,currentColor 25%,transparent);padding:.7rem;margin-bottom:.6rem}}
summary {{cursor:pointer;overflow-wrap:anywhere}}
table {{border-collapse:collapse;width:100%;margin:.75rem 0;table-layout:fixed}}
td,th {{border-bottom:1px solid color-mix(in srgb,currentColor 25%,transparent);padding:.45rem;text-align:left;vertical-align:top;overflow-wrap:anywhere}}
pre {{margin:.25rem 0;white-space:pre-wrap;overflow-wrap:anywhere}} dd {{margin-left:.65rem}}
small {{font-weight:normal;opacity:.8}} .publications {{display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,24rem),1fr));gap:.7rem}}
</style>
<main>
<h1>{title}</h1>
{body}
</main>
</html>
"""


def backend_name(report):
    name = report.get("backend", "diana")
    if not isinstance(name, str) or not name:
        raise ValueError("report backend must be a nonempty string")
    return name


def link_href(report):
    supplied = report.get("link")
    if not isinstance(supplied, str) or not supplied or supplied.startswith("/") or "\\" in supplied:
        return None
    try:
        url = urlsplit(supplied)
    except ValueError:
        return None
    return supplied if not url.scheme and not url.netloc and not url.path.startswith("/") else None


def json_identity(value):
    return json.dumps(value, sort_keys=True, ensure_ascii=True)


def series(reports):
    runs = []
    backends = {}
    for position, report in enumerate(reports):
        indexed, inputs = index_methods(report)
        backend = backend_name(report)
        profiles = {
            (item["suite"], item.get("sha256"), item.get("oracle_sha256")): item.get("corpus", {}).get("profile", "")
            for item in report["inputs"]
        }
        methods = {}
        for identity, (_, _, group) in indexed.items():
            suite, artifact, oracle, at, member = identity
            if not artifact or not oracle:
                continue
            profile = profiles.get(identity[:3], "")
            methods[(suite, artifact, at, member, profile)] = {
                (*identity, *case, json_identity(check["expected"])): check for case, check in group.items()
            }
        counts = Counter(
            check["status"] for _, _, group in indexed.values() for check in group.values()
        )
        runs.append({
            "position": position, "backend": backend, **metadata(report),
            "link": link_href(report), "checks": dict.fromkeys(STATUSES, 0) | dict(counts),
            "input_scopes": [
                {"suite": suite, "input_sha256": artifact or None, "oracle_sha256": oracle or None}
                for suite, artifact, oracle in sorted(inputs)
            ],
        })
        backends.setdefault(backend, []).append((position, methods))
    return runs, backends


def triage(backends, runs):
    latest = defaultdict(dict)
    prior = defaultdict(dict)
    for backend, history in backends.items():
        for position, methods in history:
            for method, checks in methods.items():
                previous = latest[backend].get(method)
                if previous and evaluator_compatible(runs[previous[0]], runs[position]):
                    prior[backend][method] = previous[1]
                else:
                    prior[backend].pop(method, None)
                latest[backend][method] = (position, checks)
    cases = {(method, identity) for observations in latest.values()
             for method, (_, checks) in observations.items() for identity in checks}
    mined = []
    for method, identity in sorted(cases):
        suite, artifact, oracle, at, member, metric, check_id, _ = identity
        arms = [
            {"backend": backend, "run": position, "check": checks.get(identity)}
            for backend in sorted(backends)
            for position, checks in [latest[backend].get(method, (None, {}))]
        ]
        failed = [arm["backend"] for arm in arms if arm["check"] and arm["check"]["status"] == "FAIL"]
        if not failed:
            continue
        passed = [arm["backend"] for arm in arms if arm["check"] and arm["check"]["status"] == "PASS"]
        regressions = [
            backend for backend in failed
            if prior[backend].get(method, {}).get(identity, {}).get("status") == "PASS"
        ]
        signals = []
        if regressions:
            signals.append("regression")
        if passed:
            signals.append("peer_gap")
        else:
            signals.append("all_measured_failing")
        mined.append({
            "suite": suite, "input_sha256": artifact, "oracle_sha256": oracle,
            "at": at, "member": member, "metric": metric, "id": check_id,
            **({"profile": method[-1]} if method[-1] else {}),
            "expected": json.loads(identity[-1]),
            "signals": signals, "regressions": regressions, "failing": failed, "passing": passed,
            "unmeasured": [arm["backend"] for arm in arms if arm["backend"] not in failed + passed],
            "arms": arms,
        })
    return sorted(mined, key=lambda case: (case["suite"], case["metric"], case["at"], case["id"]))


def mine_reports(reports: list[dict]) -> list[dict]:
    """Find candidate checks across the latest arms of oldest-to-newest reports."""
    runs, backends = series(reports)
    return triage(backends, runs)


def build_index(reports: list[dict]) -> dict:
    """Index supplied snapshots in oldest-to-newest order without a quality ranking."""
    runs, backends = series(reports)
    history = []
    for backend, snapshots in sorted(backends.items()):
        previous_by_selection = {}
        for current, methods in snapshots:
            selection = (json_identity(runs[current]["input_scopes"]), tuple(sorted(methods)))
            previous = previous_by_selection.get(selection)
            previous_by_selection[selection] = current
            if previous is None:
                continue
            comparison = compare_reports(reports[previous], reports[current])
            history.append({
                "backend": backend, "baseline": previous, "candidate": current,
                "summary": comparison["summary"],
            })
    return {
        "schema_version": 1, "kind": INDEX_KIND,
        "runs": runs, "history": history, "mining": triage(backends, runs),
    }


def render_index(index: dict) -> str:
    """Render grouped candidate evidence and links to historical reports."""
    require_report(index, INDEX_KIND)
    snapshots = []
    names = []
    for run in rows(index, "runs"):
        backend = label(run["backend"])
        position = run["position"]
        name = f"{backend} · {label(run.get('revision') or f'run {position + 1}')}"
        link = link_href(run)
        if link is not None:
            name = f"<a href='{label(link)}'>{name}</a>"
        names.append(name)
        counts = " · ".join(f"{label(status)} {label(count)}" for status, count in run["checks"].items())
        scopes = format_data(run["input_scopes"])
        provenance = format_data(metadata(run))
        snapshots.append(
            f"<details><summary>{name} · {counts}</summary>"
            f"<h3>Input and oracle coverage</h3><pre>{scopes}</pre>"
            f"<h3>Run provenance</h3><pre>{provenance}</pre></details>"
        )
    histories = []
    for entry in rows(index, "history"):
        baseline = names[entry["baseline"]]
        candidate = names[entry["candidate"]]
        changes = format_data(entry["summary"])
        histories.append(f"<details><summary>{baseline} → {candidate}</summary><pre>{changes}</pre></details>")
    groups = []
    candidates = list(rows(index, "mining"))
    for (suite, metric), cases in groupby(candidates, key=lambda case: (case["suite"], case["metric"])):
        cases_html = []
        for case in cases:
            arms = []
            for arm in case["arms"]:
                name = (names[arm["run"]] if arm["run"] is not None
                        else f"{label(arm['backend'])} · no matching method")
                observation = format_data(arm["check"])
                arms.append(f"<h4>{name}</h4><pre>{observation}</pre>")
            title = (
                f"<code>{label(case['member'])}</code> <small>{label(case['at'])}</small>"
                f" · {label(case['id'])} · {label(', '.join(case['signals']))}"
            )
            context = format_data({
                key: case[key] for key in (
                    "input_sha256", "oracle_sha256", "expected",
                    "regressions", "passing", "failing", "unmeasured",
                )
            })
            cases_html.append(f"<details><summary>{title}</summary><pre>{context}</pre>{''.join(arms)}</details>")
        groups.append(f"<section><h3>{label(suite)} · {label(metric)}</h3>{''.join(cases_html)}</section>")
    body = (
        "<h2>Snapshots</h2>" + ("".join(snapshots) or "<p>No snapshots.</p>")
        + "<h2>Same-backend history</h2>" + ("".join(histories) or "<p>No paired snapshots.</p>")
        + "<h2>Cases to investigate</h2>" + ("".join(groups) or "<p>No measured failures.</p>")
    )
    return html_page("Diana DecBench snapshots", body)
