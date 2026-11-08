#!/bin/sh
# Install the native CLI from a GitHub release, without building the workspace.
set -eu

fail() {
    printf 'diana installer: %s\n' "$*" >&2
    exit 1
}

case "${1:-}" in
    -h|--help)
        printf '%s\n' \
            'Usage: sh install.sh [release-tag]' \
            'Platforms: macOS/Linux, x86_64/aarch64.' \
            'DIANA_VERSION: release tag (default: latest).' \
            'DIANA_INSTALL_DIR: destination (default: $HOME/.local/bin).' \
            'Requires curl, tar, awk and standard POSIX utilities.'
        exit 0
        ;;
esac
[ "$#" -le 1 ] || fail 'expected at most one release tag; see --help'

for command in curl tar awk uname mktemp mkdir cp chmod mv rm; do
    command -v "$command" >/dev/null 2>&1 || fail "required command not found: $command"
done

case "$(uname -s)" in
    Darwin) platform=apple-darwin ;;
    Linux) platform=unknown-linux-musl ;;
    *) fail 'unsupported OS; use cargo install instead' ;;
esac
case "$(uname -m)" in
    x86_64|amd64) arch=x86_64 ;;
    arm64|aarch64) arch=aarch64 ;;
    *) fail 'unsupported architecture; use cargo install instead' ;;
esac
target="$arch-$platform"
repository=https://github.com/TsingShui/Diana
tag=${1:-${DIANA_VERSION:-latest}}

if [ "$tag" = latest ]; then
    release_url=$(curl --fail --silent --show-error --location --head \
        --proto '=https' --proto-redir '=https' --connect-timeout 15 --retry 3 \
        --output /dev/null --write-out '%{url_effective}' "$repository/releases/latest") \
        || fail 'could not resolve the latest release'
    case "$release_url" in
        "$repository/releases/tag/"*) tag=${release_url##*/} ;;
        *) fail 'could not determine a release tag; supply one explicitly' ;;
    esac
fi
case "$tag" in
    ''|-*|*[!A-Za-z0-9._-]*) fail 'release tag must contain only letters, digits, dots, underscores and hyphens' ;;
esac

if [ -n "${DIANA_INSTALL_DIR:-}" ]; then
    install_dir=$DIANA_INSTALL_DIR
else
    [ -n "${HOME:-}" ] || fail 'set HOME or DIANA_INSTALL_DIR'
    install_dir="$HOME/.local/bin"
fi

work=$(mktemp -d "${TMPDIR:-/tmp}/diana-install.XXXXXXXX")
staged=
cleanup() {
    if [ -n "$staged" ]; then rm -f -- "$staged" || :; fi
    rm -rf -- "$work" || :
}
trap cleanup 0
trap 'exit 1' 1 2 15

version=${tag#diana-}
version=${version#v}
archive="$work/release.tar.gz"
downloaded=false
# Accept stable asset names and numeric-version CLI archive names. Never select
# rasc-* or diana-gui-* assets from the same repository.
set -- "diana-$target.tar.gz"
case "$version" in
    [0-9]*) set -- "$@" "diana-$version-$target.tar.gz" "diana-v$version-$target.tar.gz" ;;
esac
for asset in "$@"; do
    url="$repository/releases/download/$tag/$asset"
    printf 'Downloading %s\n' "$url" >&2
    status=$(curl --silent --show-error --location --proto '=https' --proto-redir '=https' \
        --connect-timeout 15 --retry 3 --output "$archive" --write-out '%{http_code}' "$url") \
        || fail 'download failed'
    case "$status" in
        200) downloaded=true; break ;;
        404) ;;
        *) fail "download failed with HTTP $status" ;;
    esac
done
[ "$downloaded" = true ] || fail "no Diana CLI archive for $target in release $tag; use cargo install or choose a Diana release tag"

tar -tzf "$archive" > "$work/members" || fail 'could not read the release archive'
member=$(awk -F/ '
    $NF == "diana" {
        if ($0 !~ /^[A-Za-z0-9._+\/-]+$/ || substr($0, 1, 1) == "/") exit 1
        for (i = 1; i <= NF; i++) if ($i == "..") exit 1
        count++
        member = $0
    }
    END { if (count != 1) exit 1; print member }
' "$work/members") || fail 'archive must contain exactly one safely named diana executable'
# Extract only that member to stdout, not archive paths or symlinks onto disk.
tar -xOzf "$archive" -- "$member" > "$work/diana" || fail 'could not extract diana'
[ -s "$work/diana" ] || fail 'archive did not contain a nonempty diana executable'

mkdir -p -- "$install_dir"
install_dir=$(CDPATH= cd -- "$install_dir" && pwd -P)
[ ! -L "$install_dir/diana" ] && [ ! -d "$install_dir/diana" ] \
    || fail 'destination diana is a symbolic link or directory'
# Stage on the destination filesystem so a failed copy leaves an existing CLI intact.
staged=$(mktemp "$install_dir/.diana.XXXXXXXX")
cp -- "$work/diana" "$staged"
chmod 755 "$staged"
mv -f -- "$staged" "$install_dir/diana"
staged=
printf 'Installed %s to %s/diana\n' "$tag" "$install_dir"
case ":${PATH:-}:" in
    *":$install_dir:"*) ;;
    *) printf 'Add %s to your PATH to run diana.\n' "$install_dir" ;;
esac
