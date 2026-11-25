#!/bin/sh
# MemoIvy 마스코트 스크린캐스트 전체 재생성: 이벤트 → 음향 → 화면 → 마스터링 → 포스터·자막 → exports/, 스토어 스크린샷 → ../screenshots/
set -e
LIB=${LIB:-/home/pollux-o4/lemo-opuscar}
export LIB
HERE=$(cd "$(dirname "$0")" && pwd); WEB="$HERE/web"; OUT="$HERE/exports"
[ -f "$LIB/core/render/video.mjs" ] || { echo "LIB를 Lemo-Opuscar 라이브러리 폴더로 지정하세요"; exit 1; }
mkdir -p "$OUT"
cd "$LIB"
"$LIB/.venv/bin/python" core/audio/sampler.py --need "$WEB/score.py"   # 샘플 묶음을 긴 렌더 전에 받는다
node core/render/events.mjs "$WEB"
"$LIB/.venv/bin/python" "$WEB/mix.py"
node core/render/video.mjs "$WEB" --fps 30 --workers 4 --out "$WEB/out/video.mp4"
sh core/render/mux.sh "$WEB/out/video.mp4" "$WEB/out/mix.wav" "$OUT/memoivy.mp4" 30 0
rm "$WEB/out/video.mp4"   # 무음 중간본: 먹싱 후 불필요
node core/render/video.mjs "$WEB" --fps 30 --workers 4 --q nosubs=1 --out "$OUT/picture-only.mp4"
# 포스터: 시점은 film.js STILLS(SSoT)
"$LIB/.venv/bin/python" -c 'import json,sys; [print(e["t"], e["name"]) for e in json.load(open(sys.argv[1]))["ev"] if e["type"] == "still"]' "$WEB/events.json" |
while read -r t name; do ffmpeg -v error -y -ss "$t" -i "$OUT/memoivy.mp4" -frames:v 1 -q:v 2 "$OUT/$name.jpg"; done
node "$HERE/shots.mjs"   # 스토어 스크린샷 → ../screenshots/
echo "done → $OUT"
