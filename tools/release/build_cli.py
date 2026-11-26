#!/usr/bin/env python3
"""Build the Diana CLI with a target-native export profile."""

import argparse
import json
import os
from pathlib import Path, PurePosixPath
import struct
import subprocess
import tempfile
import zipfile


ROOT = Path(__file__).resolve().parents[2]
# These fixture methods deliberately lack the DEX initialization required for recovery.
EXPECTED_FAILURES = sorted((
    ("Lfixture/DirectInitialization;", "broken", "()Lfixture/Child;"),
    ("Lfixture/BrokenInitializer;", "<clinit>", "()V"),
    ("Lfixture/BrokenConstructor;", "<init>", "()V"),
    ("Lfixture/RecoveryWarningMembers;", "broken", "()Ljava/lang/Object;"),
))


def training_apk(path):
    tracked = subprocess.check_output(
        ["git", "ls-files", "-z", "--", ":(glob)quality/fixtures/*/*/classes.dex"], cwd=ROOT
    )
    files = sorted(PurePosixPath(os.fsdecode(name)) for name in tracked.split(b"\0") if name)
    if not files:
        raise RuntimeError("no tracked training DEX files found")
    classes = 0
    with zipfile.ZipFile(path, "w") as archive:
        for index, name in enumerate(files, 1):
            data = (ROOT / name).read_bytes()
            if len(data) < 112 or data[:4] != b"dex\n":
                raise ValueError(f"invalid DEX header in {name}")
            classes += struct.unpack_from("<I", data, 96)[0]
            entry = "classes.dex" if index == 1 else f"classes{index}.dex"
            archive.writestr(zipfile.ZipInfo(entry), data)
    return classes


def check_export(output, status, expected_classes):
    if status != 1:
        raise RuntimeError(f"expected a partial training export, got exit status {status}")
    failures = []
    report = output / "export-report.jsonl"
    for line in report.read_text(encoding="utf-8").splitlines():
        record = json.loads(line)
        if record["kind"] == "class-path":
            continue
        if record["kind"] != "methods":
            raise RuntimeError(f"unexpected export diagnostic: {record}")
        failures.extend(
            (method["class"], method["name"], method["prototype"])
            for method in record["failures"]
        )
    count = sum(
        1 for directory in ("sources", "source-definitions")
        for _ in (output / directory).rglob("*.java")
    )
    if count != expected_classes or sorted(failures) != EXPECTED_FAILURES:
        raise RuntimeError(
            f"training export wrote {count}/{expected_classes} classes and reported {failures}"
        )


def build(target, directory, profile_flag):
    env = os.environ.copy()
    env.setdefault("CARGO_PROFILE_RELEASE_LTO", "fat")
    env.setdefault("CARGO_PROFILE_RELEASE_CODEGEN_UNITS", "1")
    flags = env.get("CARGO_ENCODED_RUSTFLAGS")
    if flags is None:
        flags = "\x1f".join(env.get("RUSTFLAGS", "").split())
    env["CARGO_ENCODED_RUSTFLAGS"] = "\x1f".join(filter(None, (flags, profile_flag)))
    subprocess.run(
        ["cargo", "build", "--locked", "--release", "--target", target, "--target-dir",
         directory, "-p", "diana", "--bin", "diana"],
        cwd=ROOT, env=env, check=True,
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    host = subprocess.check_output(["rustc", "--print", "host-tuple"], text=True).strip()
    parser.add_argument("--target", default=host, help="executable Rust target (default: host)")
    args = parser.parse_args()
    sysroot = subprocess.check_output(["rustc", "--print", "sysroot"], text=True).strip()
    llvm = Path(sysroot) / "lib" / "rustlib" / host / "bin" / (
        "llvm-profdata.exe" if os.name == "nt" else "llvm-profdata"
    )
    if not llvm.is_file():
        parser.error("install the active toolchain's LLVM utility with rustup component add llvm-tools-preview")

    name = "diana.exe" if "-windows-" in args.target else "diana"
    output = ROOT / "target" / "release-pgo" / args.target
    output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=output, prefix="build-") as temp:
        work = Path(temp)
        profiles = work / "profiles"
        profiles.mkdir()
        archive = work / "training.apk"
        classes = training_apk(archive)
        cargo_output = work / "target"
        binary = cargo_output / args.target / "release" / name
        build(args.target, cargo_output, f"-Cprofile-generate={profiles}")
        train_output = work / "export"
        trained = subprocess.run(
            [binary, "export", archive, "-o", train_output, "--jobs", "2"],
            cwd=ROOT, env={**os.environ, "LLVM_PROFILE_FILE": str(profiles / "%m-%p.profraw")},
        )
        check_export(train_output, trained.returncode, classes)
        raw = sorted(profiles.glob("*.profraw"))
        if not raw:
            raise RuntimeError("instrumented CLI produced no raw profiles")
        profile = work / "training.profdata"
        subprocess.run([llvm, "merge", "-o", profile, *raw], check=True)
        build(args.target, cargo_output, f"-Cprofile-use={profile}")
        # Publish only the finished binary. Parallel invocations use independent profiles.
        binary.replace(output / name)
    print(output / name)


if __name__ == "__main__":
    main()
