#!/usr/bin/env bash
# Regenerate the quality fixtures: Java source -> classes.dex next to it.
#
# The generated DEX is committed on purpose: scoring only needs the Diana CLI and
# the Java reference, so `measure.py` runs without a JDK or an Android SDK. Run
# this script only when a fixture's Java source changes.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
QUALITY="$ROOT/quality"
WORK="$ROOT/.cache/quality"

SDK="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}"
D8=""
if [ -n "$SDK" ] && [ -d "$SDK" ]; then
    while IFS= read -r candidate; do
        if [ -x "$candidate/d8" ]; then
            D8="$candidate/d8"
        fi
    done < <(ls -1d "$SDK"/build-tools/* 2>/dev/null | sort -V)
fi
if [ -z "$D8" ]; then
    echo "error: no d8 found; set ANDROID_HOME (or ANDROID_SDK_ROOT)" >&2
    exit 1
fi
if ! command -v javac >/dev/null 2>&1; then
    echo "error: javac not found (a JDK is required to regenerate fixtures)" >&2
    exit 1
fi

echo "javac       $(javac -version 2>&1)"
echo "d8          $D8"
echo "build-tools $(basename "$(dirname "$D8")")"

for slug_dir in "$QUALITY"/fixtures/*/; do
    slug="$(basename "$slug_dir")"
    sources=("$slug_dir"*.java)
    if [ ! -e "${sources[0]}" ]; then
        echo "skip $slug: no Java source"
        continue
    fi
    out="$WORK/$slug"
    rm -rf "$out"
    mkdir -p "$out/classes" "$out/dex"
    javac --release 8 -g -d "$out/classes" "${sources[@]}"
    "$D8" --min-api 24 --output "$out/dex" $(find "$out/classes" -name '*.class')
    cp "$out/dex/classes.dex" "$slug_dir/classes.dex"
    echo "wrote quality/fixtures/$slug/classes.dex ($(wc -c < "$slug_dir/classes.dex" | tr -d ' ') bytes)"
done
