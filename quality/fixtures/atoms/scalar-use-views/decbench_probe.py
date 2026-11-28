"""Execute the source fixture's independent overload and literal scenarios."""

import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

SHA256 = "553c4bfe8381a31dec5ea577315ef49a408409e7232bac7ed982b14aff6ede56"
SCENARIOS = (
    (0, "array-value", "units", [65, 54, 53]),
    (0, "array-empty", "throw", "java.lang.ArrayIndexOutOfBoundsException"),
    (0, "array-null", "throw", "java.lang.NullPointerException"),
    (1, "boundaries", "int", 50640),
    (2, "carrier-value", "int", 3130),
    (2, "carrier-empty", "throw", "java.lang.StringIndexOutOfBoundsException"),
    (2, "carrier-null", "throw", "java.lang.NullPointerException"),
    (3, "escaped", "units", [0xD800, 0x78]),
    (4, "literal", "units", [97, 45, 98, 45]),
    (5, "parameter", "int", 1065),
)


def outcomes(observations: dict, unavailable: str, detail: str | None = None) -> dict:
    result = {}
    for method, scenario, kind, value in SCENARIOS:
        expected = {"kind": kind, "outcome": value}
        observed = observations.get(scenario)
        status = unavailable if observed is None else ("PASS" if observed == expected else "FAIL")
        row = {"id": scenario, "status": status, "expected": expected, "observed": observed}
        if detail and observed is None:
            row["detail"] = detail
        result.setdefault(f"e0:l0:c0:m{method}", []).append(row)
    return result


def invoke(argv: list[str], cwd: Path, timeout: int) -> str:
    environment = {
        key: value for key, value in os.environ.items()
        if not key.startswith("DEXDEC_")
        and key not in ("CLASSPATH", "JDK_JAVA_OPTIONS", "JAVA_TOOL_OPTIONS", "_JAVA_OPTIONS")
    }
    completed = subprocess.run(
        argv, cwd=cwd, env=environment, timeout=timeout,
        capture_output=True, check=False, encoding="utf-8",
    )
    if completed.returncode:
        raise ValueError(completed.stderr.strip() or f"command exited with {completed.returncode}")
    return completed.stdout


def probe(binary: Path, artifact: Path, timeout: int) -> dict:
    with tempfile.TemporaryDirectory(prefix="diana-scalar-judge-") as directory:
        scratch = Path(directory)
        try:
            data = artifact.read_bytes()
            if hashlib.sha256(data).hexdigest() != SHA256:
                raise ValueError("expected the authored scalar-use DEX bytes")
            input_path = scratch / "classes.dex"
            input_path.write_bytes(data)
            witness = Path(__file__).with_name("DecBenchScalar.java")
            if not witness.is_file():
                raise ValueError("missing the fixture witness")
            invoke(["java", "-version"], scratch, timeout)
        except (OSError, ValueError, subprocess.SubprocessError) as error:
            return outcomes({}, "SETUP_ERROR", str(error))
        try:
            reply = json.loads(invoke([
                str(binary), "get-class", str(input_path), "e0:l0:c0", "--format", "json",
            ], scratch, timeout))
            if (reply["schema_version"], reply["command"], reply["issues"]) != (5, "get-class", []):
                raise ValueError("incomplete class publication")
            if reply["selection"]["class_at"].split("::", 1)[0] != "e0:l0:c0":
                raise ValueError("unrelated class publication")
            source = scratch / "ScalarViews.java"
            source.write_text(reply["result"]["text"], encoding="utf-8")
            invoke([
                "java", "-m", "jdk.compiler/com.sun.tools.javac.Main", "-encoding", "UTF-8",
                "-d", str(scratch / "classes"), str(source), str(witness),
            ], scratch, timeout)
            output = invoke(["java", "-classpath", str(scratch / "classes"), "fixture.DecBenchScalar"], scratch, timeout)
            observed = {}
            for line in output.splitlines():
                scenario, kind, value = line.split("\t")
                if kind == "int":
                    value = int(value)
                elif kind == "units":
                    value = [int(unit) for unit in value.split(",")] if value else []
                elif kind != "throw":
                    raise ValueError("unexpected witness outcome")
                if scenario in observed:
                    raise ValueError("duplicate witness outcome")
                observed[scenario] = {"kind": kind, "outcome": value}
        except (OSError, KeyError, TypeError, ValueError, subprocess.SubprocessError) as error:
            return outcomes({}, "UNCHECKED", str(error))
        return outcomes(observed, "UNCHECKED", "missing named scenario from witness output")
