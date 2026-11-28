"""Exercise the recovered run body with the fault matrix from README.md."""

import json
import os
import re
import subprocess
import sys
from pathlib import Path
from tempfile import TemporaryDirectory


PREFIX = "acquire7 probe read21"
LOGGED = PREFIX + " logI"
AUDITED = LOGGED + " audit21"
CLOSED = " close21"
# Each observation names a path and an independently authored ordered trace.
SCENARIOS = (
    ("normal", (), PREFIX + CLOSED, "return", 8),
    ("acquire-throws", ("acquireI",), "acquire7", "throw", "I"),
    ("probe-throws", ("probe",), "acquire7 probe", "throw", "P"),
    ("typed-handler", ("readI",), AUDITED + CLOSED, "return", -2),
    ("log-throws", ("readI", "log"), LOGGED + CLOSED, "throw", "L"),
    ("log-and-close-throw", ("readI", "log", "close"), LOGGED + CLOSED, "throw", "C"),
    ("audit-throws", ("readI", "audit"), AUDITED, "throw", "A"),
    ("handler-close-throws", ("readI", "close"), AUDITED + CLOSED, "throw", "C"),
    ("unchecked-read", ("readR",), PREFIX + CLOSED, "throw", "R"),
    ("unchecked-read-and-close-throw", ("readR", "close"), PREFIX + CLOSED, "throw", "C"),
    ("normal-close-throws", ("close",), PREFIX + CLOSED, "throw", "C"),
)


def helper(match):
    result, name, args = match.groups()
    arg = args.split()[-1] if args else ""
    argument = '""' if not arg else f"String.valueOf({arg})"
    if name == "log":
        argument = f'({arg} == I ? "I" : "?")'
    body = f'event("{name}", {argument});'
    throws = ""
    if name in ("acquire", "read"):
        throws = " throws IOException"
        body += f' if (faults.contains("{name}I")) throw I;'
    if name == "acquire":
        body += " return 21;"
    if name == "read":
        body += ' if (faults.contains("readR")) throw R; return 8;'
    return f"public static {result} {name}({args}){throws} {{ {body} }}"


MAIN = """
    static final IOException I = new IOException("I");
    static final RuntimeException P = new RuntimeException("P"), R = new RuntimeException("R"),
        L = new RuntimeException("L"), A = new RuntimeException("A"), C = new RuntimeException("C");
    static java.util.Set<String> faults;
    static String trace;
    static void event(String helper, String arg) {
        trace += helper + arg + " ";
        if (!faults.contains(helper)) return;
        throw switch (helper) {
            case "probe" -> P; case "log" -> L; case "audit" -> A; case "close" -> C;
            default -> new IllegalArgumentException(helper);
        };
    }
    static String exitIdentity(Throwable thrown) {
        if (thrown == I) return "I";
        if (thrown == P) return "P";
        if (thrown == R) return "R";
        if (thrown == L) return "L";
        if (thrown == A) return "A";
        if (thrown == C) return "C";
        return "unexpected:" + thrown.getClass().getName();
    }
    public static void main(String[] args) {
        for (int i = 0; i < args.length; i++) {
            faults = args[i].isEmpty() ? java.util.Set.of() : java.util.Set.of(args[i].split(","));
            trace = "";
            try {
                int result = run(7);
                System.out.println(i + "\\t" + trace + "\\treturn\\t" + result);
            } catch (Throwable thrown) {
                System.out.println(i + "\\t" + trace + "\\tthrow\\t" + exitIdentity(thrown));
            }
        }
    }
"""


def harness(source: str) -> str:
    program, count = re.subn(
        r"public static native (int|void) (acquire|probe|read|log|audit|close)\(([^)]*)\)(?: throws IOException)?;",
        helper, source,
    )
    if count != 6:
        raise ValueError("expected the six original native helper declarations")
    program, count = re.subn(
        r"(public static int run\(int \w+\))(?: throws (?:IOException|Throwable))? \{",
        r"\1 throws Throwable {", program,
    )
    if count != 1:
        raise ValueError("expected the selected run declaration")
    prefix, suffix = program.rsplit("}", 1)
    return prefix + MAIN + "}" + suffix


def outcomes(status: str, detail: str, observations=None) -> list[dict]:
    observations = observations or {}
    rows = []
    for index, (name, _, trace, kind, value) in enumerate(SCENARIOS):
        expected = {"trace": trace.split(), "exit": {"kind": kind, "value": value}}
        actual = observations.get(index)
        row = {"id": name, "expected": expected, "observed": actual}
        if actual is None:
            row.update(status=status, detail=detail)
        else:
            row["status"] = "PASS" if actual == expected else "FAIL"
        rows.append(row)
    return rows


def parse_observations(stdout: str) -> dict:
    observations = {}
    for line in stdout.splitlines():
        index, trace, kind, value = line.split("\t")
        index = int(index)
        if not 0 <= index < len(SCENARIOS) or index in observations:
            raise ValueError("invalid or duplicate scenario index")
        if kind not in ("return", "throw"):
            raise ValueError("unknown scenario exit kind")
        observations[index] = {
            "trace": trace.split(),
            "exit": {"kind": kind, "value": int(value) if kind == "return" else value},
        }
    return observations


def replay(source: str, timeout: int) -> list[dict]:
    try:
        program = harness(source)
    except ValueError as error:
        return outcomes("UNCHECKED", str(error))

    with TemporaryDirectory() as tmp:
        main = Path(tmp, "HandlerCleanupBoundary.java")
        main.write_text(program, encoding="utf-8")
        environment = {
            name: value for name, value in os.environ.items()
            if name not in ("CLASSPATH", "JAVA_TOOL_OPTIONS", "JDK_JAVA_OPTIONS", "_JAVA_OPTIONS")
        }
        command_options = {
            "cwd": tmp, "env": environment, "timeout": timeout,
            "capture_output": True, "encoding": "utf-8", "errors": "replace", "check": False,
        }
        try:
            compile_result = subprocess.run(
                [
                    "java", "-m", "jdk.compiler/com.sun.tools.javac.Main",
                    "-source", "17", "-target", "17", "-Xlint:-options",
                    "-encoding", "UTF-8", "-proc:none",
                    "-classpath", tmp, "-d", tmp, str(main),
                ],
                **command_options,
            )
            if compile_result.returncode:
                diagnostics = (compile_result.stderr or compile_result.stdout).strip()
                status = (
                    "SETUP_ERROR" if any(message in diagnostics for message in (
                        "Module jdk.compiler not found", "invalid source release",
                    ))
                    else "UNCHECKED"
                )
                return outcomes(status, f"optional fixture compilation: {diagnostics}")
            execution = subprocess.run(
                [
                    "java", "--class-path", tmp, "fixture.HandlerCleanupBoundary",
                    *(",".join(faults) for _, faults, _, _, _ in SCENARIOS),
                ],
                **command_options,
            )
        except subprocess.TimeoutExpired as error:
            return outcomes("UNCHECKED", str(error))
        except OSError as error:
            return outcomes("SETUP_ERROR", str(error))

    if execution.returncode:
        return outcomes("UNCHECKED", f"fixture replay exited {execution.returncode}: {execution.stderr.strip()}")
    try:
        observations = parse_observations(execution.stdout)
    except ValueError as error:
        return outcomes("UNCHECKED", f"invalid fixture replay output: {error}")
    return outcomes("UNCHECKED", "scenario observation absent", observations)


def check(source: Path) -> int:
    rows = replay(source.read_text(encoding="utf-8"), timeout=60)
    failed = [row for row in rows if row["status"] != "PASS"]
    for row in failed:
        print(json.dumps(row), file=sys.stderr)
    print(f"scenarios={len(rows)} failed={len(failed)}")
    return bool(failed)


if __name__ == "__main__":
    sys.exit(check(Path(sys.argv[1])))
