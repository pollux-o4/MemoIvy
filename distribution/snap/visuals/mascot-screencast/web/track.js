// 타임라인 원시(film.js 전 구간 공용): 키 탐색은 이진 탐색 O(log n), 구간·엇박 등장은 한 곳에서 정의한다.
import { seg } from '/core/lib.js';

// t 이하인 마지막 키의 위치. keys는 시간 오름차순.
export function keyIndex(keys, t, time = k => k[0]) {
  let lo = 0, hi = keys.length - 1;
  while (lo < hi) { const m = (lo + hi + 1) >> 1; if (time(keys[m]) <= t) lo = m; else hi = m - 1; }
  return lo;
}
// t를 감싸는 앞뒤 키
export const around = (keys, t, time) => { const i = keyIndex(keys, t, time); return [keys[i], keys[Math.min(i + 1, keys.length - 1)]]; };
// 겹치지 않는 [시작, 끝, …] 구간 목록에서 t가 속한 구간(없으면 null)
export const spanAt = (list, t) => { const s = list[keyIndex(list, t)]; return s && t >= s[0] && t < s[1] ? s : null; };
// n개가 step 간격으로 dur 동안 차례로 등장. 화면(k)과 음향(times)이 같은 정의를 쓴다.
export function stagger(t0, n, step, dur) {
  const times = Array.from({ length: n }, (_, i) => t0 + i * step);
  return { times, end: times[n - 1] + dur, k: (t, i) => seg(t, times[i], times[i] + dur) };
}
