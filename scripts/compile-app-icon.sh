#!/usr/bin/env bash
# Compiles the Icon Composer document (src-tauri/icons/Esperanto.icon) into
# src-tauri/icons/Assets.car, the asset catalog macOS 26+ uses for the app
# icon (Liquid Glass, light/dark/tinted). Older macOS versions use icon.icns.
#
# The bundle uses the precompiled Assets.car, so `tauri build` doesn't run
# actool (and doesn't need Xcode 26). Re-run this after editing the icon.
#
# actool's .icon export fails intermittently with "Bad file descriptor", so
# the compilation is retried.
#
# Usage: scripts/compile-app-icon.sh
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
source_icon="$root/src-tauri/icons/Esperanto.icon"
target="$root/src-tauri/icons/Assets.car"
attempts=8

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
# actool names the app icon after the document; Tauri expects "Icon".
cp -R "$source_icon" "$work/Icon.icon"

for attempt in $(seq 1 "$attempts"); do
  out="$work/out-$attempt"
  mkdir -p "$out"
  # Same arguments the Tauri bundler uses.
  if xcrun actool "$work/Icon.icon" \
      --compile "$out" \
      --output-format human-readable-text \
      --output-partial-info-plist "$out/assetcatalog_generated_info.plist" \
      --app-icon Icon \
      --include-all-app-icons \
      --enable-on-demand-resources NO \
      --development-region en \
      --target-device mac \
      --minimum-deployment-target 26.0 \
      --platform macosx > "$out/actool.log" 2>&1 \
     && [[ -f "$out/Assets.car" ]]; then
    cp "$out/Assets.car" "$target"
    echo "Compiled $(basename "$source_icon") → src-tauri/icons/Assets.car (attempt $attempt)"
    exit 0
  fi
  echo "actool failed (attempt $attempt/$attempts): $(grep -m1 'error:' "$out/actool.log" || tail -1 "$out/actool.log")" >&2
  sleep 2
done

echo "Could not compile the app icon after $attempts attempts." >&2
exit 1
