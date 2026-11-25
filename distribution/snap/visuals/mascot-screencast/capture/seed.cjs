'use strict';
// 소개 영상·스크린샷 참고용 실사용 표본 데이터(.data/). 실제 앱(mise run dev:sample와 같은 엔진)이 빈 DB에서 notes.json·assets/를 가져온다.
// 사용: node seed.cjs <폴더>  — 결정적 생성(같은 UUID·좌표).
const fs = require('node:fs'), path = require('node:path'), zlib = require('node:zlib'), crypto = require('node:crypto');
const dir = path.resolve(process.argv[2] || path.join(__dirname, '.data'));
fs.rmSync(dir, { recursive: true, force: true }); fs.mkdirSync(path.join(dir, 'assets'), { recursive: true, mode: 0o700 });

// 바다 사진 대용 PNG(하늘·바다·모래 띠, 640×360)
function seaPng(w = 640, h = 360) {
  const row = y => { const b = Buffer.alloc(1 + w * 3); for (let x = 0; x < w; x++) { const k = y / h, n = Math.sin(x * .05 + y * .3) * 6;
    const c = k < .5 ? [150 + 80 * k, 205 + 30 * k, 240] : k < .78 ? [40 + n, 120 + 40 * (k - .5) + n, 175 + n] : [235, 220 - 20 * (k - .78), 170]; b.set(c.map(v => Math.max(0, Math.min(255, v | 0))), 1 + x * 3); } return b; };
  const raw = Buffer.concat(Array.from({ length: h }, (_, y) => row(y)));
  const chunk = (t, d) => { const l = Buffer.alloc(4); l.writeUInt32BE(d.length); const td = Buffer.concat([Buffer.from(t), d]); const c = Buffer.alloc(4); c.writeUInt32BE(zlib.crc32(td) >>> 0); return Buffer.concat([l, td, c]); };
  const ih = Buffer.alloc(13); ih.writeUInt32BE(w); ih.writeUInt32BE(h, 4); ih[8] = 8; ih[9] = 2;
  return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', ih), chunk('IDAT', zlib.deflateSync(raw)), chunk('IEND', Buffer.alloc(0))]);
}
const png = seaPng(), hash = crypto.createHash('sha256').update(png).digest('hex');
fs.writeFileSync(path.join(dir, 'assets', `${hash}.png`), png);

const notes = []; let seq = 0; const T0 = 1794576000;
const add = (text, { color = 'yellow', position = null, next = [], done = false, project = false, parent = null, pinned = false } = {}) => {
  const id = `e0000000-0000-4000-8000-${(++seq).toString(16).padStart(12, '0')}`;
  const canvas = { position, next, done }; if (project) canvas.project = true; if (parent) canvas.parent = parent; if (pinned) canvas.pinned = true;
  notes.push({ id, text, color, modified: T0 - notes.length * 1800, canvas }); return id;
};
const L = (...a) => a.join('\n');

// 프로젝트 1: 주말 여행 — 연결·체크·진행 중·링크·사진
const trip = add(L('주말 여행', '부산 1박 2일, 토요일 아침 출발'), { project: true, pinned: true, color: 'yellow', position: [80, 80] });
const train = add(L('기차표 예매', '토요일 **09:10** 출발, 창가 두 자리', '- [x] 왕복 예매', '- [x] 좌석 지정'), { parent: trip, color: 'blue', position: [520, 60] });
const stay = add(L('숙소', '체크인 15시 · 주차 가능한지 물어보기', 'https://example.com/stay'), { parent: trip, color: 'green', position: [520, 360] });
const photo = add(L('여행 사진', `![바다|480](asset:${hash})`, '바다 사진은 따로 묶고 열 장만 남기기'), { parent: trip, color: 'pink', position: [940, 220] });
add(L('# 주말 여행 준비', '- [x] 기차표 예매', '- [x] 숙소 전화', '- [/] 짐 싸기', '- [ ] 우산 챙기기', '', '**토요일 09:10** 서울역 집합'), { parent: trip, color: 'yellow', position: [80, 200], next: [train, stay] });
notes.find(n => n.id === stay).canvas.next.push(photo);
add(L('맛집 후보', '- 돼지국밥', '- 밀면', '- 어묵 거리'), { parent: trip, color: 'purple', position: [940, 520] });

// 프로젝트 2: 블로그 글 — 마인드맵 가지
const blog = add(L('블로그 글: 메모 정리법', '이번 주 금요일 발행'), { project: true, color: 'blue', position: [520, 80] });
const b1 = add(L('자료 모으기', '- [x] 참고 글 3개', '- [ ] 스크린샷'), { parent: blog, color: 'blue', position: [480, 60] });
const b2 = add(L('초안', '## 들어가며', '메모는 *쓰는 것*보다 **다시 찾는 것**이 어렵다.'), { parent: blog, color: 'green', position: [480, 300] });
const b3 = add(L('퇴고', '- [ ] 맞춤법', '- [ ] 제목 세 개 후보'), { parent: blog, color: 'pink', position: [480, 540] });
add(L('메모 정리법', '글의 뼈대'), { parent: blog, color: 'yellow', position: [80, 300], next: [b1, b2, b3] });

// 프로젝트 3: 이사 준비 — 완료가 섞인 진행
const move = add(L('이사 준비', '12월 둘째 주'), { project: true, color: 'green', position: [960, 80] });
['인터넷 이전 신청', '전입 신고', '짐 박스 20개', '가구 배치도'].forEach((t, i) => add(L(t, i < 2 ? '- [x] 완료' : '- [ ] 아직'), { parent: move, color: ['green', 'blue', 'yellow', 'purple'][i], position: [80 + (i % 2) * 360, 80 + Math.floor(i / 2) * 240], done: i < 2 }));

// 낱장 메모
add(L('회의 3시', '분기 계획 공유', '- [ ] 자료 전날 정리'), { color: 'blue' });
add(L('장보기', '- [ ] 우유', '- [x] 달걀', '- [ ] 사과', '- [ ] 커피 원두'), { color: 'green' });
add(L('세금 납부', '이번 달 25일까지, 자동이체 확인'), { color: 'purple' });
add(L('읽을 책', '- 『작은 습관의 힘』', '- 『생각의 탄생』'), { color: 'pink' });
add(L('운동 7시', '하체 · 스트레칭 10분'), { color: 'yellow' });

fs.writeFileSync(path.join(dir, 'notes.json'), JSON.stringify(notes, null, 2));
fs.writeFileSync(path.join(dir, 'assets.json'), JSON.stringify({ auto_prune: false, assets: { [hash]: { added: T0, last_note: photo } } }));
console.log(`메모 ${notes.length}개 → ${dir}`);
