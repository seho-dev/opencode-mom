#!/bin/sh
set -eu

icons=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
root=$(CDPATH= cd -- "$icons/../.." && pwd)
output=$(mktemp -d "$icons/.generated.XXXXXX")
trap 'rm -rf "$output"' EXIT HUP INT TERM

node "$root/node_modules/@tauri-apps/cli/tauri.js" icon "$icons/app-icon.svg" --output "$output/app"
cp "$output/app/"*.png "$output/app/icon.ico" "$output/app/icon.icns" "$icons/"

for name in tray-macos tray-windows; do
  node "$root/node_modules/@tauri-apps/cli/tauri.js" icon "$icons/$name.svg" --output "$output/$name" --png 16 --png 32 --png 64
  cp "$output/$name/16x16.png" "$icons/$name-16.png"
  cp "$output/$name/32x32.png" "$icons/$name.png"
  cp "$output/$name/64x64.png" "$icons/$name@2x.png"
done
