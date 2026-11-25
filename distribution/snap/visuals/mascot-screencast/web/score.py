"""배경음악 악보: 실제 악기 샘플(전부 CC0: FreePats·VSCO 2 CE·VCSL)로 '종이와 식물' 콘셉트.
cue(film.js 타임라인)와 BEAT를 받아 (N,2) 스테레오를 돌려준다. 효과음·믹스는 mix.py."""
import os, sys
import numpy as np
sys.path.insert(0, os.environ['LIB'])
from core.audio import sampler as S
from core.audio.sfx import SR, lp

# 화성: F장조 I–vi–IV–V(한 마디 4박). 행드럼(D단조)과 같은 음계라 함께 쓴다.
PROG = [('F2', ['F3', 'A3', 'C4', 'E4']), ('D2', ['D3', 'F3', 'A3', 'C4']), ('Bb1', ['Bb2', 'D3', 'F3', 'A3']), ('C2', ['C3', 'E3', 'G3', 'Bb3'])]
MEL = ['C5', 'A4', 'F5', 'E5', 'D5', 'F5', 'A5', 'G5']   # 두 마디짜리 글로켄 동기(마디당 4음, 2박 간격)
INSTRUMENTS = ['kalimba', 'ukulele', 'contrabass_pizz', 'violins_pizz', 'cellos_pizz', 'glockenspiel', 'marimba', 'flute',
               'violins', 'cellos', 'hand_chimes', 'harp', 'glass', 'shaker', 'cajon', 'woodblock', 'triangle']


def score(cue, BEAT, dur):
    BAR = 4 * BEAT; ev = []; S.seed(11)
    e = lambda t, i, p, d, v, pan=0, g=1: ev.append((t, i, p, d, v, pan, g))
    kal = lambda p: S.midi(p) + (12 if S.midi(p) + 12 <= S.midi('C#5') else 0)   # 칼림바 녹음 음역(F3–C#5) 안에서만
    bars = lambda a, b: [(a + k * BAR, k) for k in range(int((b - a) / BAR + .5))]

    # 1막 혼돈: 낮은 피치카토가 반음으로 맴돌고, 나무토막이 엇박으로 재촉
    for k, p in enumerate(['D3', 'Eb3', 'D3', 'C#3'] * 3):
        t = cue['chaos'] + .2 + k * BEAT
        if t < cue['hush'] - .2: e(t, 'cellos_pizz', p, BEAT, .55, -.2); e(t + BEAT / 2, 'violins_pizz', S.midi(p) + 15, BEAT / 2, .3, .3)
    for k in range(10): e(cue['chaos'] + .45 + k * BEAT * 1.25, 'woodblock', None, None, .25, .4)
    # 숨: 유리 하모니카 한 음이 열리기 직전을 받친다
    e(cue['hush'], 'glass', 'A5', 1.4, .35, 0, .7)

    # 2막 공개: 칼림바 아르페지오 + 우쿨렐레 + 콘트라베이스 + 셰이커
    for t, k in bars(cue['reveal'], cue['connect']):
        root, ch = PROG[k % 4]
        e(t, 'contrabass_pizz', root, BEAT * 2, .7, -.1); e(t + 2 * BEAT, 'contrabass_pizz', S.midi(root) + 7, BEAT * 2, .55, -.1)
        for j in range(8): e(t + j * BEAT / 2, 'kalimba', kal(ch[[0, 2, 1, 3][j % 4]]), BEAT, .5 if j % 2 == 0 else .38, (j % 4 - 1.5) * .25)
        for j in range(8): e(t + j * BEAT / 2, 'shaker', 'down' if j % 2 == 0 else 'up', None, .22 if j % 2 else .3, .35)
        for j in (1, 3): ev.extend((t + j * BEAT + i * .018, 'ukulele', p, BEAT * .8, .45, (i - 1.5) * .3, 1) for i, p in enumerate(PROG[k % 4][1]))

    # 3막 잇기: 박이 단단해지고(카혼) 글로켄이 동기를 노래, 마림바가 대답
    for t, k in bars(cue['connect'], cue['done']):
        root, ch = PROG[k % 4]
        for j, d in ((0, 1.5), (1.5, .5), (2, 1.5), (3.5, .5)): e(t + j * BEAT, 'contrabass_pizz', root if j < 2 else S.midi(root) + 7, d * BEAT, .7, -.1)
        for j in range(8): e(t + j * BEAT / 2, 'kalimba', kal(ch[[0, 2, 1, 3][j % 4]]), BEAT, .42, (j % 4 - 1.5) * .25)
        for j in range(4): e(t + j * BEAT, 'cajon', 'bass' if j % 2 == 0 else 'slap', None, .5 if j % 2 == 0 else .38, 0)
        for j in range(8): e(t + j * BEAT / 2, 'shaker', 'down' if j % 2 == 0 else 'up', None, .2, .35)
        for j in range(4): e(t + j * BEAT, 'glockenspiel', S.midi(MEL[(k % 2) * 4 + j]) + 12, BEAT * 1.5, .42, .2)
        e(t + 3.5 * BEAT, 'marimba', S.midi(ch[-1]) + 12, BEAT, .4, -.35)
    # 완료: 하프 글리산도 + 트라이앵글 + 핸드차임
    for i, p in enumerate(['F4', 'A4', 'C5', 'E5', 'F5', 'A5', 'C6', 'F6']): e(cue['done'] + i * .045, 'harp', p, 2, .55, -.3 + i * .08)
    e(cue['done'], 'triangle', 'open', None, .4, .3)
    for i, p in enumerate(['F4', 'A4', 'C5']): e(cue['done'] + .4 + i * .3, 'hand_chimes', p, 2.5, .45, (i - 1) * .3)
    for t, k in bars(cue['done'] + 1.6, cue['grow']):
        root, ch = PROG[k % 4]; e(t, 'contrabass_pizz', root, BAR, .55)
        for j in range(4): e(t + j * BEAT, 'kalimba', kal(ch[j]), BEAT * 1.5, .38, (j - 1.5) * .25)

    # 4막 자람: 현 패드가 깔리고 플루트가 동기를 길게 부른다
    for t, k in bars(cue['grow'], cue['pre']):
        root, ch = PROG[k % 4]; d = min(BAR, cue['pre'] - t)
        e(t, 'cellos', root, d, .4, -.2); ev.extend((t, 'violins', S.midi(p) + 12, d, .3, (i - 1) * .3, .7) for i, p in enumerate(ch[1:]))
        for j in range(8): e(t + j * BEAT / 2, 'kalimba', kal(ch[[0, 2, 1, 3][j % 4]]), BEAT, .35, (j % 4 - 1.5) * .25)
        e(t, 'flute', MEL[(k % 2) * 4], BEAT * 1.9, .42, .15); e(t + 2 * BEAT, 'flute', MEL[(k % 2) * 4 + 2], BEAT * 1.9, .42, .15)
        for j in range(4): e(t + j * BEAT + BEAT / 2, 'shaker', 'down', None, .15, .35)

    # 엔드: 하프·글로켄·첼로로 F 화음을 길게 울린다
    for i, p in enumerate(['F2', 'C3', 'F3', 'A3', 'C4', 'E4', 'A4']): e(cue['end'] + i * .06, 'harp', p, 5, .5, (i - 3) * .12)
    e(cue['end'], 'cellos', 'F2', 4.5, .35); e(cue['end'] + .5, 'glockenspiel', 'F5', 4, .4, .2); e(cue['end'] + .9, 'glockenspiel', 'C6', 4, .32, -.2)

    out = S.render([x for x in ev if x[0] < dur], dur, master=False)
    out = S.room(out, size=.45, mix=.2)
    a, n = int(cue['chaos'] * SR), int(cue['hush'] * SR)                     # 혼돈: 종이 더미 속에 갇힌 소리
    out[a:n] = np.stack([lp(out[a:n, c], 900) for c in (0, 1)], 1)
    a0, n0 = int(cue['pre'] * SR), int(.9 * SR)                              # 엔드 카드 직전 정적
    out[a0:a0 + n0] *= np.linspace(1, 0, n0)[:, None]; out[a0 + n0:int(cue['end'] * SR)] = 0
    return out
