#!/usr/bin/env bash
# Prepare every fixture: assemble Smali, or compile sources and then dex classes.
#
# Smali, when present, is the assembly input; paired Java explains its behavior.
# Otherwise any mix of Java and Kotlin is compiled together and dexed once.
# The generated DEX is committed
# on purpose, so a review needs no toolchain - this script is only for the author
# who changed a fixture.
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

KOTLINC="${KOTLINC:-}"
if [ -z "$KOTLINC" ]; then
    KOTLINC="/Applications/Android Studio.app/Contents/plugins/Kotlin/kotlinc/bin/kotlinc"
fi
KOTLIN_LIB="$(dirname "$KOTLINC")/../lib/kotlin-stdlib.jar"
R8_JAR="${R8_JAR:-/Applications/Android Studio.app/Contents/plugins/android/lib/r8.jar}"
PROGUARD_JARS="${PROGUARD_JARS:-$(ls "$ROOT"/.cache/tools/proguard/*.jar 2>/dev/null | paste -sd: -)}"
ANDROID_JAR="${ANDROID_JAR:-$SDK/platforms/android-37.0/android.jar}"

echo "d8          $D8"
echo "build-tools $(basename "$(dirname "$D8")")"
echo "javac       $(javac -version 2>&1)"
if [ -x "$KOTLINC" ]; then
    echo "kotlinc     $("$KOTLINC" -version 2>&1 | head -1)"
else
    echo "kotlinc     not found (only needed by fixtures with .kt sources)"
fi

for slug_dir in "$QUALITY"/fixtures/*/*/; do
    slug="$(basename "$slug_dir")"
    out="$WORK/$slug"
    rm -rf "$out"
    mkdir -p "$out/classes" "$out/dex"

    smali_sources=("$slug_dir"*.smali)
    if [ -e "${smali_sources[0]}" ]; then
        if [ -z "${SMALI_CLASSPATH:-}" ]; then
            echo "error: $slug needs a local Smali assembler; set SMALI_CLASSPATH" >&2
            exit 1
        fi
        smali_api=24
        if grep -Eq '^[[:space:]]*invoke-custom(/range)?[[:space:]]' "${smali_sources[@]}"; then
            smali_api=28
        fi
        java -cp "$SMALI_CLASSPATH" org.jf.smali.Main assemble --api "$smali_api" \
            -o "$out/assembled.dex" "$slug_dir"
        if [ "$smali_api" -eq 28 ]; then
            # Smali 2.5's writer omits invoke-custom from outgoing-word sizing.
            # Correct code_item metadata from the assembled instruction operands.
            mkdir -p "$WORK/tools"
            javac -cp "$SMALI_CLASSPATH" -d "$WORK/tools" "$QUALITY/NormalizeSmali.java"
            java -cp "$WORK/tools:$SMALI_CLASSPATH" NormalizeSmali \
                "$out/assembled.dex" "$out/dex/classes.dex"
        else
            cp "$out/assembled.dex" "$out/dex/classes.dex"
        fi
        cp "$out/dex/classes.dex" "$slug_dir/classes.dex"
        echo "wrote ${slug_dir#"$QUALITY"/}classes.dex ($(wc -c < "$slug_dir/classes.dex" | tr -d ' ') bytes)"
        continue
    fi

    java_sources=("$slug_dir"*.java)
    kotlin_sources=("$slug_dir"*.kt)

    # A fixture may pin its own compiler arguments (an indy fixture needs a newer
    # release and/or `--no-desugaring`); the default is the shared Java 8 baseline.
    javac_args=(--release 8 -g)
    if [ -f "$slug_dir/javac.args" ]; then
        read -r -a javac_args < "$slug_dir/javac.args"
    fi
    d8_args=(--min-api 24)
    if [ -f "$slug_dir/d8.args" ]; then
        read -r -a d8_args < "$slug_dir/d8.args"
    fi

    if [ -e "${java_sources[0]}" ]; then
        javac "${javac_args[@]}" -d "$out/classes" "${java_sources[@]}"
    fi
    if [ -e "${kotlin_sources[0]}" ]; then
        if [ ! -x "$KOTLINC" ]; then
            echo "error: $slug has Kotlin sources but kotlinc is missing" >&2
            exit 1
        fi
        "$KOTLINC" -jvm-target 1.8 -d "$out/classes" "${kotlin_sources[@]}"
    fi

    class_files=$(find "$out/classes" -name '*.class')
    if [ -z "$class_files" ]; then
        echo "error: $slug produced no classes" >&2
        exit 1
    fi

    # Compiled-source fixtures use their own rules file for optional obfuscation.
    rules="$slug_dir/rules.pro"
    if [ -f "$rules" ]; then
        (
            cd "$out"
            mkdir -p input && cp -r classes/. input/ && (cd input && zip -qr ../classes.jar .)
            if grep -q '^# obfuscator: *proguard' "$rules"; then
                if [ -z "$PROGUARD_JARS" ]; then
                    echo "error: $slug needs ProGuard jars; set PROGUARD_JARS" >&2
                    exit 1
                fi
                cp "$rules" proguard.pro
                printf -- "-injars %s/classes.jar\n-outjars %s/obfuscated.jar\n-libraryjars %s\n" \
                    "$out" "$out" "$ANDROID_JAR" >> proguard.pro
                java -cp "$PROGUARD_JARS" proguard.ProGuard @proguard.pro
                mkdir -p obfuscated && (cd obfuscated && unzip -qo ../obfuscated.jar)
                "$D8" --min-api 24 --output dex $(find obfuscated -name '*.class')
            else
                java -cp "$R8_JAR" com.android.tools.r8.R8 --release --min-api 24 \
                    --lib "$ANDROID_JAR" --pg-conf "$rules" --output dex classes.jar
            fi
        )
        if [ -f "$out/mapping.txt" ]; then
            cp "$out/mapping.txt" "$slug_dir/mapping.txt"
        fi
    elif [ -f "$KOTLIN_LIB" ]; then
        "$D8" "${d8_args[@]}" --lib "$KOTLIN_LIB" --output "$out/dex" $class_files
    else
        "$D8" "${d8_args[@]}" --output "$out/dex" $class_files
    fi
    cp "$out/dex/classes.dex" "$slug_dir/classes.dex"
    echo "wrote ${slug_dir#"$QUALITY"/}classes.dex ($(wc -c < "$slug_dir/classes.dex" | tr -d ' ') bytes)"
done
