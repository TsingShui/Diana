"""Create a local CFR launcher for DecBench's authored standalone JVM corpus.

The launcher inventories the original class with javap and publishes full-class
source directly from a user-supplied CFR JAR. It exposes neither method text nor
CFR-recovered member/type facts. The shared behavioral probes use get-class.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile


JAVAP = ("java", "-m", "jdk.jdeps/com.sun.tools.javap.Main", "-p", "-s")
DECLARATION = re.compile(r"^(?:[a-z]+ )*(?:class|interface) ([\w.$]+)(?=[\s<{])")
JAVA_ENV_OPTIONS = ("CLASSPATH", "JAVA_TOOL_OPTIONS", "_JAVA_OPTIONS", "JDK_JAVA_OPTIONS")


def file_digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def run_java(argv: list[str], timeout: int) -> str:
    environment = {key: value for key, value in os.environ.items() if key not in JAVA_ENV_OPTIONS}
    result = subprocess.run(
        argv, check=False, capture_output=True, encoding="utf-8", errors="replace",
        env=environment, timeout=timeout,
    )
    if result.returncode:
        raise ValueError(result.stderr.strip() or f"Java command exited with status {result.returncode}")
    if result.stderr:
        sys.stderr.write(result.stderr)
    return result.stdout


def parse_inventory(disassembly: str) -> tuple[str, list[dict]]:
    """Read only original descriptors and method-table order from javap."""
    owner = None
    items = []
    declaration = ""
    for raw in disassembly.splitlines():
        if not raw.startswith(" ") and (match := DECLARATION.match(raw)):
            if owner is not None:
                raise ValueError("javap emitted more than one class declaration")
            owner = match[1]
        line = raw.strip()
        if line.startswith("descriptor: ("):
            if owner is None:
                raise ValueError("javap emitted a method descriptor without its class")
            prototype = line.removeprefix("descriptor: ")
            if declaration == "static {};":
                name = "<clinit>"
            else:
                header, opening, _ = declaration.partition("(")
                words = header.split()
                if not opening or not words:
                    raise ValueError("javap did not identify the method before its descriptor")
                name = "<init>" if words[-1] == owner else words[-1]
            items.append({
                "at": f"j0:m{len(items)}",
                "member": f"L{owner.replace('.', '/')};->{name}{prototype}",
            })
        if line:
            declaration = line
    if owner is None:
        raise ValueError("javap did not identify an original class")
    return f"L{owner.replace('.', '/')};", items


def parser_for_launcher() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="CFR standalone JVM fixture backend")
    commands = parser.add_subparsers(dest="command", required=True)
    for command in ("methods", "get-class", "get-method"):
        request = commands.add_parser(command)
        request.add_argument("artifact", type=Path)
        request.add_argument("--format", choices=("json",), required=True)
        if command == "methods":
            request.add_argument("--no-limit", action="store_true", required=True)
        else:
            request.add_argument("at")
    return parser


def serve(jar: Path, jar_hash: str, argv: list[str], timeout: int) -> int:
    args = parser_for_launcher().parse_args(argv)
    try:
        artifact = args.artifact.resolve(strict=True)
        if not artifact.is_file() or artifact.suffix != ".class":
            raise ValueError("the CFR adapter covers standalone JVM .class files")
        if args.command == "get-method":
            raise ValueError("the CFR adapter publishes class source only; method publication is unmeasured")
        if file_digest(jar) != jar_hash:
            raise ValueError("the CFR JAR has changed; create a new launcher")
        owner, items = parse_inventory(run_java([*JAVAP, str(artifact)], timeout))
        if args.command == "methods":
            response = {
                "schema_version": 2, "command": "methods", "complete": True,
                "items": items, "provenance": "original class file via javap",
            }
        else:
            if args.at != "j0":
                raise ValueError("the standalone class selector is j0")
            text = run_java(["java", "-jar", str(jar), str(artifact)], timeout)
            if not text.strip():
                raise ValueError("CFR emitted no class source")
            response = {
                "schema_version": 5, "command": "get-class",
                "selection": {"class": owner, "class_at": "j0", "input_kind": "jvm"},
                "result": {"text": text}, "issues": [],
            }
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print(str(error), file=sys.stderr)
        return 2
    print(json.dumps(response, ensure_ascii=True))
    return 0


def create_launcher(jar: Path, launcher: Path, timeout: int = 60) -> None:
    jar = jar.resolve(strict=True)
    if not jar.is_file():
        raise ValueError("expected a CFR JAR file")
    adapter = Path(__file__).resolve()
    contents = f"""#!/usr/bin/env python3
from hashlib import file_digest
from pathlib import Path
import runpy
import sys

adapter = Path({str(adapter)!r})
with adapter.open("rb") as stream:
    if file_digest(stream, "sha256").hexdigest() != {file_digest(adapter)!r}:
        raise SystemExit("CFR adapter changed; create a new launcher")
serve = runpy.run_path(str(adapter))["serve"]
raise SystemExit(serve(Path({str(jar)!r}), {file_digest(jar)!r}, sys.argv[1:], {timeout!r}))
"""
    launcher = launcher.absolute()
    launcher.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="cfr-launcher-", dir=launcher.parent) as directory:
        temporary = Path(directory) / "launcher"
        temporary.write_text(contents, encoding="utf-8")
        temporary.chmod(0o755)
        os.link(temporary, launcher)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--jar", type=Path, required=True)
    parser.add_argument("--launcher", type=Path, required=True)
    parser.add_argument("--timeout", type=int, default=60)
    args = parser.parse_args()
    if args.timeout < 1:
        parser.error("--timeout must be positive")
    try:
        create_launcher(args.jar, args.launcher, args.timeout)
    except (OSError, ValueError) as error:
        print(f"cfr adapter: {error}", file=sys.stderr)
        return 2
    print(args.launcher.absolute())
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
