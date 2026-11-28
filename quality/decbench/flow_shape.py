"""Compare normalized statement control flow without judging semantic equivalence."""

from collections import Counter
import json
import os
from pathlib import Path
import re
import subprocess
from tempfile import TemporaryDirectory

from evidence import run_cli


PARSER = Path(__file__).with_name("FlowShape.java")
SOURCE_BYTES = 4 * 1024 * 1024
MATCH_BUDGET = 20_000
DESCRIPTOR = re.compile(r"\[*(?:[BCDFIJSZ]|L[^;]+;)")


def request(member, owner):
    prefix, sep, signature = member.partition("->")
    name, opening, tail = signature.partition("(")
    parameters, closing, result = tail.partition(")")
    descriptors = DESCRIPTOR.findall(parameters)
    if (not sep or prefix != owner or not name or not opening or not closing
            or not result or "".join(descriptors) != parameters):
        raise ValueError(f"invalid method descriptor for structural probe: {member}")
    return (name, str(len(descriptors)))


def graph_contract(graph):
    return [
        (node["kind"], Counter(edge for edge, _ in node["edges"])) for node in graph
    ]


def equal_graphs(reference, observed):
    """Find a rooted bijection, permitting polarity swaps at opaque binary tests."""
    if reference == observed:
        return True
    if len(reference) != len(observed):
        return False
    before, after = graph_contract(reference), graph_contract(observed)
    if Counter(kind for kind, _ in before) != Counter(kind for kind, _ in after):
        return False
    stack = [([(0, 0)], {}, {})]
    steps = 0
    while stack:
        pending, forward, reverse = stack.pop()
        while pending:
            steps += 1
            if steps > MATCH_BUDGET:
                return None
            left, right = pending.pop()
            if left in forward:
                if forward[left] != right:
                    break
                continue
            if right in reverse or before[left] != after[right]:
                break
            forward[left], reverse[right] = right, left
            old_edges = reference[left]["edges"]
            new_edges = observed[right]["edges"]
            if before[left][0] == "test":
                old_a, old_b = [target for _, target in old_edges]
                new_a, new_b = [target for _, target in new_edges]
                alternatives = []
                for a, b in ((new_a, new_b), (new_b, new_a)):
                    pairs = [(old_a, a), (old_b, b)]
                    if all(before[x] == after[y] and forward.get(x, y) == y
                           and reverse.get(y, x) == x for x, y in pairs):
                        if pairs not in alternatives:
                            alternatives.append(pairs)
                if not alternatives:
                    break
                if len(alternatives) == 2:
                    if len(stack) >= 128:
                        return None
                    stack.append((pending + alternatives[1], forward.copy(), reverse.copy()))
                pending.extend(alternatives[0])
            else:
                pending.extend((a, b) for (_, a), (_, b) in zip(old_edges, new_edges))
        else:
            return True
    return False


def switch_targets(graph):
    switches = [node for node in graph if node["kind"] == "switch"]
    return dict(switches[0]["edges"]) if len(switches) == 1 else {}


def reaches(graph, start, goal):
    seen, remaining = set(), [start]
    while remaining:
        target = remaining.pop()
        if target == goal:
            return True
        if target not in seen and graph[target]["kind"] != "switch":
            seen.add(target)
            remaining.extend(index for _, index in graph[target]["edges"])
    return False


def display_label(label):
    if label.startswith("case:"):
        _, kind, value = label.split(":", 2)
        return f"case {value!r}" if kind in ("STRING_LITERAL", "CHAR_LITERAL") else f"case {value}"
    return label


def divergence(reference, observed):
    old_cases, new_cases = switch_targets(reference), switch_targets(observed)
    if old_cases and old_cases.keys() == new_cases.keys() and len(old_cases) < 128:
        for label, start in old_cases.items():
            for other, target in old_cases.items():
                if label == other:
                    continue
                if start == target and new_cases[label] != new_cases[other]:
                    return (f"authored {display_label(label)} and {display_label(other)} share one entry, "
                            "while the recovered entries are distinct")
                if (start != target and reaches(reference, start, target)
                        and not reaches(observed, new_cases[label], new_cases[other])):
                    return (f"authored {display_label(label)} reaches the {display_label(other)} entry, "
                            "and the recovered graph lacks this path")
    return "normalized statement control flow differs from the authored reference"


def parse_graphs(reference, recovered, owner, methods, timeout):
    if reference.stat().st_size > SOURCE_BYTES or recovered.stat().st_size > SOURCE_BYTES:
        raise ValueError("source exceeds the structural probe size bound")
    arguments = [str(reference), str(recovered), owner[1:-1]]
    for method in methods:
        arguments.extend(request(method["member"], owner))
    environment = {
        key: value for key, value in os.environ.items()
        if key not in {"CLASSPATH", "_JAVA_OPTIONS", "JDK_JAVA_OPTIONS", "JAVA_TOOL_OPTIONS"}
    }
    process = subprocess.run(
        ["java", "-Xmx256m", str(PARSER), *arguments],
        capture_output=True, text=True, encoding="utf-8", env=environment,
        timeout=timeout, check=False,
    )
    if process.returncode:
        raise ValueError(process.stderr.strip() or f"Java parser exited {process.returncode}")
    sides = [{}, {}]
    for line in process.stdout.rstrip("\n").split("\n"):
        side, index, kind, payload = line.split("\t", 3)
        i, j, result = int(side), int(index), json.loads(payload)
        if i not in (0, 1) or not 0 <= j < len(methods) or j in sides[i]:
            raise ValueError("unexpected Java parser report identity")
        if kind == "graph" and isinstance(result, list) and result:
            sides[i][j] = {"graph": result}
        elif kind == "unchecked" and isinstance(result, str):
            sides[i][j] = {"detail": result}
        else:
            raise ValueError("unexpected Java parser report contents")
    if any(len(side) != len(methods) for side in sides):
        raise ValueError("Java parser report is incomplete")
    return sides


def check(status, expected=None, observed=None, detail=None):
    result = {"id": "cfg", "status": status, "expected": expected, "observed": observed}
    if detail:
        result["detail"] = detail
    return result


def compare(binary: Path, artifact: Path, manifest: dict, timeout: int) -> dict[str, list[dict]]:
    methods = [method for method in manifest["methods"] if method.get("structure")]
    if not methods:
        return {}
    config = manifest["structure"]
    directory = Path(manifest["manifest"]).resolve().parent
    reference = (directory / config["reference"]).resolve()
    if not reference.is_relative_to(directory) or not reference.is_file():
        return {row["at"]: [check("SETUP_ERROR", detail="missing authored structural source")] for row in methods}
    owner, location = config["class_member"], config["class_at"]
    if not owner.startswith("L") or not owner.endswith(";"):
        return {row["at"]: [check("SETUP_ERROR", detail="invalid structural class owner")] for row in methods}
    publication = run_cli(
        [str(binary), "get-class", str(artifact), location, "--format", "json"], timeout,
    )
    value = publication.get("publication", {})
    selection, result = value.get("selection"), value.get("result")
    if (publication["exit_code"] not in (0, 1)
            or value.get("schema_version") != 5 or value.get("command") != "get-class"
            or not isinstance(selection, dict)
            or not isinstance(selection.get("class_at"), str)
            or selection["class_at"].split("::", 1)[0] != location
            or isinstance(selection.get("class"), str) and selection["class"] != owner
            or not isinstance(result, dict) or not isinstance(result.get("text"), str)):
        detail = publication.get("error", "class-source observation was unavailable")
        return {row["at"]: [check("UNCHECKED", detail=detail)] for row in methods}
    try:
        with TemporaryDirectory(prefix="decbench-flow-") as tmp:
            recovered = Path(tmp) / "Recovered.java"
            recovered.write_text(result["text"], encoding="utf-8")
            authored, rendered = parse_graphs(reference, recovered, owner, methods, timeout)
    except subprocess.TimeoutExpired:
        return {row["at"]: [check("UNCHECKED", detail="structural probe timed out")] for row in methods}
    except (OSError, ValueError) as error:
        return {row["at"]: [check("SETUP_ERROR", detail=str(error))] for row in methods}

    result = {}
    for index, method in enumerate(methods):
        left, right = authored[index], rendered[index]
        expected, observed = left.get("graph"), right.get("graph")
        if expected is None:
            verification = check("UNCHECKED", observed=observed,
                                 detail=f"authored structural reference unavailable: {left['detail']}")
        elif observed is None:
            verification = check("UNCHECKED", expected=expected, detail=right["detail"])
        else:
            agreement = equal_graphs(expected, observed)
            if agreement is None:
                verification = check("UNCHECKED", expected, observed, "graph comparison reached its search bound")
            elif agreement:
                verification = check("PASS", expected, observed)
            else:
                verification = check("FAIL", expected, observed, divergence(expected, observed))
        result[method["at"]] = [verification]
    return result
