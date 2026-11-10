#!/usr/bin/env bash
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"
if [[ -n "$(git status --porcelain --untracked-files=all)" ]]; then
  echo "배포 중단: 변경·미추적 파일을 먼저 검토하고 커밋하세요." >&2
  exit 1
fi

mise run check:secrets
mise run test:snap-gtk
mise run package:snap

version="$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)"
shopt -s nullglob
snap_files=("memoivy_${version}_"*.snap)

if [[ -z "$version" || "${#snap_files[@]}" -ne 1 ]]; then
  echo "Expected one Snap package for version ${version:-unknown}; found ${#snap_files[@]}." >&2
  exit 1
fi

snapcraft upload --release=stable "${snap_files[0]}"
