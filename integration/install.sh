#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
temp_dir=$(mktemp -d)
trap 'rm -rf "$temp_dir"' EXIT
install -Dm755 target/release/linux-sticky-memo "${HOME}/.local/bin/linux-sticky-memo"
gcc -Wall -Wextra -O2 "-DUSER_HOME=\"${HOME}\"" integration/launcher.c -o "$temp_dir/launcher"
install -Dm755 "$temp_dir/launcher" "${HOME}/.local/libexec/linux-sticky-memo-launcher"
install -Dm644 integration/linux-sticky-memo.svg "${HOME}/.local/share/icons/hicolor/scalable/apps/linux-sticky-memo.svg"
sed "s|@HOME@|${HOME}|g" integration/linux-sticky-memo.desktop > "$temp_dir/desktop"
install -Dm644 "$temp_dir/desktop" "${HOME}/.local/share/applications/io.github.lsm.LinuxStickyMemo.desktop"
link="${HOME}/바탕화면/서비스 실행 파일/Linux Sticky Memo"
mkdir -p "$(dirname "$link")"
if [ ! -L "$link" ] || [ "$(readlink "$link")" != "${HOME}/.local/libexec/linux-sticky-memo-launcher" ]; then
    ln -s "${HOME}/.local/libexec/linux-sticky-memo-launcher" "$link"
fi
gio set "$link" metadata::custom-icon "file://${HOME}/.local/share/icons/hicolor/scalable/apps/linux-sticky-memo.svg"
update-desktop-database "${HOME}/.local/share/applications" 2>/dev/null || true
