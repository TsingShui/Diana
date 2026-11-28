"""Report source and recovered statement CFGs as public-corpus review evidence."""

from __future__ import annotations

from pathlib import Path
from tarfile import TarError

from corpus_oracles import IDENTIFIER, Unavailable, member_parts, read_sources, verify_source
from flow_shape import (
    BATCH_BYTES, BATCH_UNITS, SOURCE_BYTES, ParseUnit, divergence, equal_graphs, parse_batch,
)


def method_unit(fragment: str, owner: str) -> str:
    """Supply the selected owner for a method fragment without rewriting its body."""
    parts = owner[1:-1].split("/")
    if not all(IDENTIFIER.fullmatch(part) for part in parts):
        raise Unavailable("the original owner cannot delimit a Java syntax wrapper")
    package = "package " + ".".join(parts[:-1]) + ";\n" if len(parts) > 1 else ""
    return package + "class " + parts[-1] + " {\n" + fragment + "\n}\n"


def parse_unit(text, owner, member, fragment):
    if fragment:
        text = method_unit(text, owner)
    data = text.encode("utf-8")
    if len(data) > SOURCE_BYTES:
        raise Unavailable("selected method input exceeds the structural probe size bound")
    return ParseUnit(owner, member, data)


def graph_batches(units, timeout):
    batch = []
    indices = []
    size = 0
    for index, unit in units:
        length = len(unit.source)
        if len(batch) >= BATCH_UNITS or size + length > BATCH_BYTES:
            yield from zip(indices, parse_batch(batch, timeout))
            batch, indices, size = [], [], 0
        indices.append(index)
        batch.append(unit)
        size += length
    if batch:
        yield from zip(indices, parse_batch(batch, timeout))


def receipt(expected, observed, outcome, detail=None):
    result = {"id": "cfg", "outcome": outcome, "expected": expected, "observed": observed}
    if detail:
        result["detail"] = detail
    return result


def compare_pair(before, after):
    expected, observed = before.get("graph"), after.get("graph")
    if problem := before.get("setup_error") or after.get("setup_error"):
        return receipt(expected, observed, "SETUP_ERROR", problem)
    if expected is None:
        return receipt(None, observed, "UNCHECKED",
                       "original source CFG unavailable: " + before["detail"])
    if observed is None or after.get("partial"):
        return receipt(expected, observed, "UNCHECKED", after.get("partial") or after["detail"])
    agreement = equal_graphs(expected, observed)
    if agreement is None:
        return receipt(expected, observed, "UNCHECKED", "graph comparison reached its search bound")
    if agreement:
        return receipt(expected, observed, "MATCH")
    return receipt(expected, observed, "DIFFERENT", divergence(expected, observed))


def inspect(cases, source_archive: Path, source_root: str, observations, timeout: float,
            *, backend: str = "diana") -> dict[str, dict]:
    """Project verified upstream method spans and selected Diana or JADX publications.

    A structural delta directs review. This parser ignores most expressions and
    does not model exception handling or other evaluation effects.
    """
    methods = list(cases)
    try:
        files = read_sources(source_archive, source_root, {case["source"]["path"] for case in methods})
        source_error = None
    except (OSError, ValueError, TarError) as error:
        files = {}
        source_error = str(error)

    results = {}
    jobs = []
    partial = {}
    for case in methods:
        at, member = case["at"], case["member"]
        owner, _, _ = member_parts(member)
        sides = [{}, {}]
        results[at] = sides
        source = case["source"]
        data = files.get(source["path"])
        if source_error is not None:
            sides[0] = {"setup_error": source_error}
        else:
            try:
                verify_source(case, data)
                lines = data.decode("utf-8").splitlines(keepends=True)
                text = "".join(lines[source["start_line"] - 1:source["end_line"]])
                jobs.append(((at, 0), parse_unit(text, owner, member, True)))
            except (Unavailable, UnicodeError) as error:
                sides[0] = {"detail": str(error)}
            except ValueError as error:
                sides[0] = {"setup_error": str(error)}

        publication = observations.get(at) or {}
        text = publication.get("text")
        error = publication.get("error")
        if isinstance(text, str) and text.strip():
            try:
                jobs.append(((at, 1), parse_unit(text, owner, member,
                                                backend == "jadx" or publication.get("fragment", False))))
            except (Unavailable, UnicodeError) as problem:
                sides[1] = {"detail": str(problem)}
            else:
                if error:
                    partial[at] = str(error)
        else:
            sides[1] = {"detail": str(error or "selected method publication is unavailable")}

    for (at, side), graph in graph_batches(jobs, timeout):
        results[at][side] = graph
    for at, problem in partial.items():
        results[at][1]["partial"] = problem
    return {at: compare_pair(*sides) for at, sides in results.items()}
