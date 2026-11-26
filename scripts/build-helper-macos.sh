#!/usr/bin/env bash
# Builds the macOS native helper and places it where Tauri expects sidecars.
set -euo pipefail
cd "$(dirname "$0")/.."
[[ "$(uname)" == "Darwin" ]] || { echo "skip: not macOS"; mkdir -p src-tauri/binaries; exit 0; }
mkdir -p native/macos/build src-tauri/binaries
SRC=native/macos/MinPointHelper/main.swift
FW="-framework AppKit -framework Vision -framework AVFoundation -framework Carbon"
for arch in arm64 x86_64; do
  swiftc -O -swift-version 5 -target ${arch}-apple-macos13.0 "$SRC" -o native/macos/build/minpoint-helper-$arch $FW
done
lipo -create native/macos/build/minpoint-helper-arm64 native/macos/build/minpoint-helper-x86_64 -output native/macos/build/minpoint-helper
cp native/macos/build/minpoint-helper-arm64 src-tauri/binaries/minpoint-helper-aarch64-apple-darwin
cp native/macos/build/minpoint-helper-x86_64 src-tauri/binaries/minpoint-helper-x86_64-apple-darwin
cp native/macos/build/minpoint-helper src-tauri/binaries/minpoint-helper-universal-apple-darwin
echo "helper built: $(du -h native/macos/build/minpoint-helper | cut -f1) universal"
