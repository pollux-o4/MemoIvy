#!/usr/bin/env bash
set -euo pipefail

version="$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)"
shopt -s nullglob
snap_files=("memoivy_${version}_"*.snap)

if [[ -z "$version" || "${#snap_files[@]}" -ne 1 ]]; then
  echo "Expected one Snap package for version ${version:-unknown}; found ${#snap_files[@]}." >&2
  exit 1
fi

snapcraft upload --release=stable "${snap_files[0]}"
