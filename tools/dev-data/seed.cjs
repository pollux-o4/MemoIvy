'use strict';
// 개발용 데이터 폴더(.dev-data)를 표본 메모로 새로 만든다. 사용자 실데이터 폴더는 건드리지 않는다.
// 엔진은 빈 DB에서 처음 시작할 때 notes.json을 가져온다(legacy import).
const fs = require('node:fs');
const path = require('node:path');
const { sampleNotes } = require('./sample_notes.cjs');

const DEV_DATA_DIR = path.resolve(__dirname, '../../.dev-data');
const reset = process.argv.includes('--reset');

if (fs.existsSync(DEV_DATA_DIR) && !reset) {
  console.log(`표본 데이터가 이미 있습니다: ${DEV_DATA_DIR} (새로 만들려면 mise run dev:sample:reset)`);
} else {
  fs.rmSync(DEV_DATA_DIR, { recursive: true, force: true });
  fs.mkdirSync(DEV_DATA_DIR, { recursive: true, mode: 0o700 });
  const notes = sampleNotes();
  fs.writeFileSync(path.join(DEV_DATA_DIR, 'notes.json'), JSON.stringify(notes, null, 2));
  console.log(`표본 메모 ${notes.length}개를 만들었습니다: ${DEV_DATA_DIR}`);
}
