#!/usr/bin/env bash
# Compiles the generated Kotlin declarations with warnings as errors and round-trips the
# Serde JSON samples against the Kotlinx output. See README.md.
set -euo pipefail

here="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
manifest="$here/../../Cargo.toml"

KOTLINX_SERIALIZATION_VERSION="${KOTLINX_SERIALIZATION_VERSION:-1.13.0}"
KOTLINC="${KOTLINC:-kotlinc}"

if ! command -v "$KOTLINC" >/dev/null 2>&1; then
    echo "error: '$KOTLINC' was not found; install Kotlin or set KOTLINC" >&2
    exit 1
fi
KOTLINC="$(command -v "$KOTLINC")"

resolve() {
    if readlink -f "$1" >/dev/null 2>&1; then
        readlink -f "$1"
    else
        printf '%s\n' "$1"
    fi
}

# The serialization compiler plugin ships with the compiler distribution. Homebrew keeps the
# real distribution under `libexec`, while the official archive puts it under `lib`.
kotlin_home="$(CDPATH= cd -- "$(dirname -- "$(resolve "$KOTLINC")")/.." && pwd)"
plugin=""
for candidate in \
    "${KOTLIN_HOME:+$KOTLIN_HOME/lib}" \
    "$kotlin_home/lib" \
    "$kotlin_home/libexec/lib"; do
    if [ -n "$candidate" ] && [ -f "$candidate/kotlinx-serialization-compiler-plugin.jar" ]; then
        plugin="$candidate/kotlinx-serialization-compiler-plugin.jar"
        break
    fi
done
if [ -z "$plugin" ]; then
    echo "error: could not find kotlinx-serialization-compiler-plugin.jar; set KOTLIN_HOME" >&2
    exit 1
fi

kotlin="${KOTLIN:-$(dirname -- "$KOTLINC")/kotlin}"

lib="$here/lib"
mkdir -p "$lib"
artifact() {
    local name="$1"
    local path="$lib/kotlinx-serialization-$name-jvm-$KOTLINX_SERIALIZATION_VERSION.jar"
    if [ ! -f "$path" ]; then
        echo "downloading $name $KOTLINX_SERIALIZATION_VERSION" >&2
        local artifact_path="org/jetbrains/kotlinx/kotlinx-serialization-$name-jvm/$KOTLINX_SERIALIZATION_VERSION/kotlinx-serialization-$name-jvm-$KOTLINX_SERIALIZATION_VERSION.jar"
        curl --fail --silent --show-error --location --retry 3 \
            --output "$path.tmp" "https://repo.maven.apache.org/maven2/$artifact_path" ||
            curl --fail --silent --show-error --location --retry 3 \
                --output "$path.tmp" "https://repo1.maven.org/maven2/$artifact_path" || return
        mv "$path.tmp" "$path" || return
    fi
    printf '%s\n' "$path"
}
# Separate assignments preserve download failures instead of masking the first substitution's status.
core="$(artifact core)"
json="$(artifact json)"
classpath="$core:$json"

echo "generating Kotlin declarations"
cargo run --quiet --manifest-path "$manifest" -p specta-tests --example kotlin_typecheck

generated="$here/generated"
build="$here/build"
rm -rf "$build"

compile() {
    local config="$1"
    shift
    mkdir -p "$build/$config"
    "$KOTLINC" -Werror -Xplugin="$plugin" -classpath "$classpath" -d "$build/$config" "$@"
}

echo "compiling Serialization::None"
compile none "$generated/none/Bindings.kt"
compile none-unknown "$generated/none-unknown/Bindings.kt"
compile none-wide "$generated/none-wide/Bindings.kt"

echo "compiling Serialization::Kotlinx"
compile kotlinx "$generated/kotlinx/Bindings.kt" "$here/Support.kt" "$here/Known.kt"
compile kotlinx-unknown "$generated/kotlinx-unknown/Bindings.kt" "$here/Support.kt" "$here/Unknown.kt"

samples="$generated/samples.json"
echo "round-tripping Serde JSON samples"
"$kotlin" -classpath "$build/kotlinx:$classpath" dev.specta.kotlincheck.KnownKt "$samples"
"$kotlin" -classpath "$build/kotlinx-unknown:$classpath" dev.specta.kotlincheck.UnknownKt "$samples"
