#!/usr/bin/env bash
# Downloads the official llama.cpp build (macOS arm64) and prepares
# llama-server to be bundled as a Tauri sidecar.
#
# Output (ignored by git):
#   src-tauri/binaries/llama-server-aarch64-apple-darwin   the executable
#   src-tauri/binaries/lib/*.dylib                          its libraries
#   src-tauri/binaries/bundle.conf.json                     Tauri config that
#       bundles them; pass it to `tauri build --config` (the Taskfile does).
#
# The executable looks for its libraries in ../Frameworks (where Tauri puts
# `bundle.macOS.frameworks`) and next to itself, and is ad-hoc re-signed
# after that change.
#
# Usage: scripts/fetch-llama-server.sh
set -euo pipefail

BUILD="b12359"
ASSET="llama-${BUILD}-bin-macos-arm64.tar.gz"
URL="https://github.com/ggml-org/llama.cpp/releases/download/${BUILD}/${ASSET}"
SHA256="82b00abc740160e1b9980e6cc79c117345fa34af058c80e6c0a69aa8ddccde75"
TRIPLE="aarch64-apple-darwin"

root="$(cd "$(dirname "$0")/.." && pwd)"
out="$root/src-tauri/binaries"
target="$out/llama-server-$TRIPLE"
stamp="$out/.llama-build"

if [[ -x "$target" && -f "$stamp" && "$(cat "$stamp")" == "$BUILD" ]]; then
  echo "llama-server $BUILD already present."
  exit 0
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "Downloading $ASSET…"
curl -fL --progress-bar -o "$tmp/$ASSET" "$URL"
echo "$SHA256  $tmp/$ASSET" | shasum -a 256 -c -

mkdir -p "$tmp/x"
tar -xzf "$tmp/$ASSET" -C "$tmp/x"
server="$(find "$tmp/x" -type f -name llama-server -perm -u+x | head -1)"
[[ -n "$server" ]] || { echo "llama-server not found in $ASSET" >&2; exit 1; }
bin_dir="$(dirname "$server")"

rm -rf "$out"
mkdir -p "$out/lib"
cp "$server" "$target"

# Copy only the libraries actually loaded (following @rpath references
# recursively), as regular files under the name they are referenced by.
queue=("$target")
while ((${#queue[@]})); do
  current="${queue[0]}"; queue=("${queue[@]:1}")
  for name in $(otool -L "$current" | awk '/@rpath\//{sub("@rpath/","",$1); print $1}'); do
    if [[ ! -f "$out/lib/$name" ]]; then
      cp -L "$bin_dir/$name" "$out/lib/$name"
      chmod u+w "$out/lib/$name"
      queue+=("$out/lib/$name")
    fi
  done
done

# Look for the libraries in Contents/Frameworks inside the app bundle.
install_name_tool -add_rpath "@executable_path/../Frameworks" "$target" 2>/dev/null || true
codesign --force --sign - "$target" >/dev/null
# Tauri config merged at build time: the sidecar goes to Contents/MacOS,
# the libraries to Contents/Frameworks.
{
  printf '{\n  "bundle": {\n    "externalBin": ["binaries/llama-server"],\n    "macOS": {\n      "frameworks": [\n'
  first=1
  for lib in "$out"/lib/*.dylib; do
    ((first)) || printf ',\n'
    printf '        "binaries/lib/%s"' "$(basename "$lib")"
    first=0
  done
  printf '\n      ]\n    }\n  }\n}\n'
} > "$out/bundle.conf.json"

echo "$BUILD" > "$stamp"

echo "Prepared $(basename "$target") and $(ls "$out/lib" | wc -l | tr -d ' ') libraries in $out"
