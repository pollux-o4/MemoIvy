#!/usr/bin/env bash
# Snap 배포본과 같은 GTK(gnome 확장 런타임)로 테스트를 돌린다.
# 테스트 바이너리는 시스템 GTK로 빌드하고, 실행할 때 libgtk만 런타임 것으로 바꿔 끼운다.
# 인자는 테스트 이름 필터로 넘어간다. 예: tools/quality/test-snap-gtk.sh fmt_003
set -euo pipefail

runtime="${MEMOIVY_SNAP_GTK_RUNTIME:-/snap/gnome-46-2404/current}"
lib="$(ls "$runtime"/usr/lib/x86_64-linux-gnu/libgtk-4.so.1.* 2>/dev/null | head -n 1 || true)"
if [[ -z "$lib" ]]; then
  echo "Snap GTK 런타임을 찾지 못했다: $runtime (snap install gnome-46-2404)" >&2
  exit 1
fi

overlay="target/snap-gtk"
mkdir -p "$overlay"
ln -sf "$lib" "$overlay/libgtk-4.so.1"
echo "GTK: $(basename "$lib") ($runtime)" >&2

binary="$(cargo test --locked --no-run --message-format=json 2>/dev/null \
  | sed -n 's/.*"executable":"\([^"]*\)".*"fresh".*/\1/p;s/.*"executable":"\([^"]*\)".*/\1/p' \
  | grep '/deps/memoivy-' | head -n 1)"
if [[ -z "$binary" ]]; then
  echo "테스트 바이너리를 찾지 못했다. cargo test --no-run 출력을 확인한다." >&2
  exit 1
fi

exec dbus-run-session -- xvfb-run -a env LD_LIBRARY_PATH="$PWD/$overlay" "$binary" "$@"
