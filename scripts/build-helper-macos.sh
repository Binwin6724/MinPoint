#!/usr/bin/env bash
# Builds the macOS native helper and places it where Tauri expects sidecars.
set -euo pipefail
cd "$(dirname "$0")/.."
[[ "$(uname)" == "Darwin" ]] || { echo "skip: not macOS"; mkdir -p src-tauri/binaries; exit 0; }
mkdir -p native/macos/build src-tauri/binaries
SRC=native/macos/InputFlowHelper/main.swift
FW="-framework AppKit -framework Vision -framework AVFoundation -framework Carbon"
for arch in arm64 x86_64; do
  swiftc -O -swift-version 5 -target ${arch}-apple-macos13.0 "$SRC" -o native/macos/build/inputflow-helper-$arch $FW
done
lipo -create native/macos/build/inputflow-helper-arm64 native/macos/build/inputflow-helper-x86_64 -output native/macos/build/inputflow-helper
cp native/macos/build/inputflow-helper-arm64 src-tauri/binaries/inputflow-helper-aarch64-apple-darwin
cp native/macos/build/inputflow-helper-x86_64 src-tauri/binaries/inputflow-helper-x86_64-apple-darwin
cp native/macos/build/inputflow-helper src-tauri/binaries/inputflow-helper-universal-apple-darwin
echo "helper built: $(du -h native/macos/build/inputflow-helper | cut -f1) universal"
