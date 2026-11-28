"""Run opt-in authored models of selected Java-compatible method publications."""

from dataclasses import dataclass
import json
from pathlib import Path
import re
import runpy
import subprocess
from tempfile import TemporaryDirectory

import corpus_oracles
from judge import QUALITY, check, child_path
import tool_capture


MARKER = "/* DECBENCH_SELECTED_METHOD */"
MAX_MODEL_BYTES = 64 * 1024
MAX_SCENARIOS = 32
JAVA = ["java", "-XX:+IgnoreUnrecognizedVMOptions", "-XX:-UsePerfData"]


@dataclass(frozen=True)
class Model:
    manifest: Path
    template_path: Path
    template: str
    class_name: str
    scenarios: list[dict]
    raw_probe: Path | None


def read_model(location: str) -> Model:
    manifest = child_path(QUALITY / "fixtures", location)
    with manifest.open("rb") as stream:
        contents = stream.read(MAX_MODEL_BYTES + 1)
    if len(contents) > MAX_MODEL_BYTES:
        raise ValueError(f"{manifest}: model manifest exceeds its byte bound")
    config = json.loads(contents)
    if (not isinstance(config, dict) or config.get("schema_version") != 1
            or not isinstance(config.get("class_name"), str)
            or not re.fullmatch(r"[A-Za-z_$][\w$]*", config["class_name"])
            or not isinstance(config.get("template"), str) or not config["template"]
            or not isinstance(config.get("scenarios"), list)
            or not 0 < len(config["scenarios"]) <= MAX_SCENARIOS):
        raise ValueError(f"{manifest}: invalid public execution model")
    ids = set()
    for spec in config["scenarios"]:
        if not isinstance(spec, dict) or not isinstance(spec.get("id"), str) or not spec["id"] or spec["id"] in ids or any(
            not isinstance(spec.get(key), list) or any(not isinstance(value, str) for value in spec[key])
            for key in ("arguments", "expected")
        ):
            raise ValueError(f"{manifest}: invalid or duplicate scenario")
        ids.add(spec["id"])
    template_path = child_path(manifest.parent, config["template"])
    with template_path.open("rb") as stream:
        template = stream.read(MAX_MODEL_BYTES + 1).decode("utf-8")
    if len(template.encode("utf-8")) > MAX_MODEL_BYTES or template.count(MARKER) != 1:
        raise ValueError(f"{manifest}: expected one insertion marker in a bounded template")
    raw_probe = None
    if "raw_probe" in config:
        name = config["raw_probe"]
        if not isinstance(name, str) or not re.fullmatch(r"[a-z][a-z0-9_]*\.py", name):
            raise ValueError(f"{manifest}: invalid raw DEX probe filename")
        raw_probe = child_path(manifest.parent, name)
        if not raw_probe.is_file() or raw_probe.stat().st_size > MAX_MODEL_BYTES:
            raise ValueError(f"{manifest}: the raw DEX probe is missing or exceeds the model size bound")
    return Model(manifest, template_path, template, config["class_name"], config["scenarios"], raw_probe)


def raw_controls(model: Model, artifact: Path, *, at: str, member: str) -> dict:
    if model.raw_probe is None:
        return {}
    scenarios = {spec["id"]: spec for spec in model.scenarios}

    def unavailable(identifier, detail):
        return {"id": identifier, "status": "SETUP_ERROR", "expected": scenarios[identifier]["expected"],
                "observed": None, "trace": [], "detail": detail}

    try:
        probe = runpy.run_path(str(model.raw_probe)).get("probe")
        results = probe(artifact, at=at, member=member) if callable(probe) else None
        if not isinstance(results, list) or len(results) > MAX_SCENARIOS:
            raise ValueError("the raw DEX probe must return a bounded list of scenario receipts")
    except (OSError, ValueError, ImportError, KeyError, TypeError, SyntaxError) as error:
        return {identifier: unavailable(identifier, str(error)) for identifier in scenarios}

    checked = {}
    for result in results:
        identifier = result.get("id") if isinstance(result, dict) else None
        if not isinstance(identifier, str) or identifier not in scenarios:
            continue
        status = result.get("status")
        expected = scenarios[identifier]["expected"]
        if (identifier in checked or status not in ("PASS", "UNCHECKED", "SETUP_ERROR")
                or result.get("expected") != expected or not isinstance(result.get("trace"), list)
                or status == "PASS" and result.get("observed") != expected):
            checked[identifier] = unavailable(identifier, "the raw DEX receipt does not match the authored scenario")
        else:
            checked[identifier] = result
    return {identifier: checked[identifier] if identifier in checked else unavailable(
        identifier, "the raw DEX probe returned no receipt for this scenario",
    ) for identifier in scenarios}


def attach_raw_controls(checks, controls):
    for row in checks:
        control = controls.get(row["id"])
        if control is None:
            continue
        row["original_dex"] = control
        if control["status"] == "SETUP_ERROR":
            row["status"] = "SETUP_ERROR"
            row["detail"] = control.get("detail") or "the original DEX control failed"
    return checks


def method_source(text: str, member: str) -> str:
    view = corpus_oracles.View(text)
    opening, closing = view.method_body(member)
    first, last = view.tokens[opening].line - 1, view.tokens[closing].line
    _, name, _ = corpus_oracles.member_parts(member)
    declaration = re.compile(rf"^[\t ]*(?:public|protected|private)\s+(?:static\s+)?[^;{{}}]+\b{re.escape(name)}\s*\(")
    lines = text.split("\n")
    for start in range(first, max(-1, first - 12), -1):
        if declaration.match(lines[start]):
            return "\n".join(lines[start:last])
    raise corpus_oracles.Unavailable("the selected Java method declaration could not be recovered")


def execute(model: Model, body: str, scratch: Path, timeout: int) -> list[tuple[list[str] | None, str | None]]:
    scratch.mkdir()
    source = scratch / f"{model.class_name}.java"
    source.write_text(model.template.replace(MARKER, body), encoding="utf-8")
    compiler = [*JAVA, "-Xmx128m", "-m", "jdk.compiler/com.sun.tools.javac.Main", "-proc:none",
                "-encoding", "UTF-8", "-classpath", str(scratch), "-d", str(scratch), str(source)]
    result = invoke(compiler, scratch, timeout)
    if result.returncode:
        raise ValueError(diagnostics(result.stderr, scratch) or "the model did not compile")
    events = []
    runtime = [*JAVA, "-Xmx64m", "-classpath", str(scratch), model.class_name]
    for spec in model.scenarios:
        try:
            observed = invoke(runtime + spec["arguments"], scratch, timeout)
        except subprocess.TimeoutExpired:
            events.append((None, f"model scenario timed out after {min(timeout, 15):g} seconds"))
        except tool_capture.OutputLimit as error:
            events.append((None, str(error)))
        else:
            output = observed.stdout.decode("utf-8", "replace").splitlines()
            if observed.returncode:
                detail = diagnostics(observed.stderr, scratch)
                events.append((None, detail or f"model exited {observed.returncode}"))
            elif len(output) != 1:
                events.append((None, "the model did not emit exactly one event trace"))
            else:
                events.append((output[0].split(",") if output[0] else [], None))
    return events


def diagnostics(output: bytes, scratch: Path) -> str:
    return output.decode("utf-8", "replace").replace(str(scratch), "<model>")[:1500]


def invoke(argv, scratch, timeout):
    return tool_capture.run(argv, cwd=scratch, timeout=min(timeout, 15),
                            output_limit=64 * 1024, diagnostic_limit=64 * 1024)


def compare(model: Model, original: str | None, observation: dict | None, member: str, timeout: int,
            *, source_detail: str | None = None) -> list[dict]:
    def row(spec, status, observed=None, detail=None):
        return check(spec["id"], "modeled_behavior", status, spec["expected"], observed, detail)

    if original is None:
        detail = source_detail or "the pinned source body is unavailable"
        return [row(spec, "SETUP_ERROR", detail=detail) for spec in model.scenarios]
    with TemporaryDirectory(prefix="decbench-model-") as directory:
        scratch = Path(directory)
        try:
            control = execute(model, method_source(original, member), scratch / "original", timeout)
        except (OSError, ValueError, subprocess.SubprocessError) as error:
            return [row(spec, "SETUP_ERROR", detail=str(error)) for spec in model.scenarios]
        unavailable = "the selected recovered method is unavailable"
        recovered = [(None, unavailable) for _ in model.scenarios]
        if observation and not observation.get("error") and observation.get("text"):
            try:
                recovered = execute(model, method_source(observation["text"], member), scratch / "observed", timeout)
            except (OSError, ValueError, subprocess.SubprocessError) as error:
                recovered = [(None, str(error)) for _ in model.scenarios]
    rows = []
    for spec, (expected, source_error), (observed, recovery_error) in zip(model.scenarios, control, recovered):
        if source_error or expected != spec["expected"]:
            rows.append(row(spec, "SETUP_ERROR", expected, source_error or "the source control disagrees with the scenario"))
        elif recovery_error:
            rows.append(row(spec, "UNCHECKED", detail=recovery_error))
        else:
            rows.append(row(spec, "PASS" if observed == expected else "FAIL", observed))
    return rows
