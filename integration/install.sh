#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
temp_dir=$(mktemp -d)
stage=""
trap 'rm -rf "$temp_dir"; if [ -n "$stage" ]; then rm -rf "$stage"; fi' EXIT
build_dir="artifacts/installers/linux-unpacked"
if [ ! -x "$build_dir/memoivy" ]; then
    echo "먼저 mise run package:dir 로 Linux 배포 폴더를 생성하세요." >&2
    exit 1
fi
mkdir -p "${HOME}/.local/lib" "${HOME}/.local/bin"
stage=$(mktemp -d "${HOME}/.local/lib/.memoivy-install.XXXXXX")
cp -a "$build_dir/." "$stage/"
app_dir="${HOME}/.local/lib/memoivy-desktop"
rm -rf "$app_dir.previous"
if [ -e "$app_dir" ] || [ -L "$app_dir" ]; then
    mv "$app_dir" "$app_dir.previous"
fi
mv "$stage" "$app_dir"
stage=""
ln -sfn "$app_dir/memoivy" "${HOME}/.local/bin/memoivy"
gcc -Wall -Wextra -O2 "-DUSER_HOME=\"${HOME}\"" integration/launcher.c -o "$temp_dir/launcher"
install -Dm755 "$temp_dir/launcher" "${HOME}/.local/libexec/memoivy-launcher"
install -Dm644 integration/memoivy.svg "${HOME}/.local/share/icons/hicolor/scalable/apps/io.github.pollux_o4.MemoIvy.svg"
sed "s|@HOME@|${HOME}|g" integration/memoivy.desktop > "$temp_dir/desktop"
install -Dm644 "$temp_dir/desktop" "${HOME}/.local/share/applications/io.github.pollux_o4.MemoIvy.desktop"
link="${HOME}/바탕화면/서비스 실행 파일/MemoIvy"
mkdir -p "$(dirname "$link")"
if [ -e "$link" ] && [ ! -L "$link" ]; then
    echo "설치 중단: $link 경로에 심볼릭 링크가 아닌 파일이 있습니다." >&2
    exit 1
fi
if [ -L "$link" ] && [ "$(readlink "$link")" != "${HOME}/.local/libexec/memoivy-launcher" ]; then
    echo "설치 중단: $link 심볼릭 링크가 MemoIvy 실행기를 가리키지 않습니다." >&2
    exit 1
fi
if [ ! -L "$link" ]; then
    ln -s "${HOME}/.local/libexec/memoivy-launcher" "$link"
fi
gio set "$link" metadata::custom-icon "file://${HOME}/.local/share/icons/hicolor/scalable/apps/io.github.pollux_o4.MemoIvy.svg"

# Remove prior app launchers only after the new MemoIvy installation is ready.
old_link="${HOME}/바탕화면/서비스 실행 파일/Linux Sticky Memo"
if [ -L "$old_link" ] && [ "$(readlink "$old_link")" = "${HOME}/.local/libexec/linux-sticky-memo-launcher" ]; then
    rm "$old_link"
fi
rm -f \
    "${HOME}/.local/bin/linux-sticky-memo" \
    "${HOME}/.local/libexec/linux-sticky-memo-launcher" \
    "${HOME}/.local/share/applications/io.github.lsm.LinuxStickyMemo.desktop" \
    "${HOME}/.local/share/icons/hicolor/scalable/apps/linux-sticky-memo.svg"
update-desktop-database "${HOME}/.local/share/applications" 2>/dev/null || true
