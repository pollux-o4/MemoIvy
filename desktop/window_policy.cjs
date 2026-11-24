'use strict';
// 창 정책 SSoT: 목록·캔버스는 홈 자리 하나를 나눠 쓰고(home), 메모는 작업 창(work), 그 외는 보조 창(aux)이다.
// 목록은 닫힌 창 뒤에 홈·작업 창이 하나도 보이지 않을 때만 다시 뜬다(보조 창은 아무 창도 없을 때만).

function roleOf(key) {
  if (key === 'library' || key === 'canvas') return 'home';
  return key.startsWith('note:') ? 'work' : 'aux';
}

function homeToReplace(openingKey, openKeys) {
  if (roleOf(openingKey) !== 'home') return null;
  for (const key of openKeys) {
    if (key !== openingKey && roleOf(key) === 'home') return key;
  }
  return null;
}

function shouldShowLibrary(closingKey, visibleKeys) {
  if (closingKey === 'library') return false;
  const closingRole = roleOf(closingKey);
  for (const key of visibleKeys) {
    if (closingRole === 'aux' || roleOf(key) !== 'aux') return false;
  }
  return true;
}

module.exports = { roleOf, homeToReplace, shouldShowLibrary };
