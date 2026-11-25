"""음악·효과음·앰비언스를 events.json(film.js 타임라인)에서 합성해 out/mix.wav로 쓴다.
음악은 score.py(실제 악기 샘플), 칩튠 사각파는 잎새의 목소리다."""
import json, os, sys
from pathlib import Path
import numpy as np
from scipy.io.wavfile import write
sys.path.insert(0, os.path.join(os.environ['LIB'], 'core/audio'))
import sfx
from sfx import SR, add, t_, env_exp, lp, hp, bp, noise, norm, brown
sys.path.insert(0, str(Path(__file__).parent))
from score import score

HERE = Path(__file__).parent
E = json.loads((HERE / 'events.json').read_text())
DUR, EV = E['dur'], E['ev']
tempo = next((e for e in EV if e['type'] == 'tempo'), None)
if tempo is None or 'beat' not in tempo:
    raise ValueError("events.json requires a tempo event with 'beat' in seconds")
BEAT = tempo['beat']
cue = {e['id']: e['t'] for e in EV if e['type'] == 'cue'}
N = int(DUR * SR)
fx, amb = (np.zeros((N, 2)) for _ in range(2))
rng = np.random.default_rng(7)
hz = lambda m: 440 * 2 ** ((m - 69) / 12)

def square(f, d, v=.12, duty=.25, slide=0.):   # 잎새의 목소리(칩)
    tt = t_(d); fr = f * (1 + slide * tt / d)
    ph = np.cumsum(fr) / SR % 1
    x = np.where(ph < duty, 1., -1.)
    return lp(x, 5000) * np.minimum(1, tt / .004) * np.clip((d - tt) / .02, 0, 1) * v

music = score(cue, BEAT, DUR)

# ── 효과음 ──
SCALE = [72, 74, 76, 79, 81, 84, 86, 88, 91, 93, 96, 98]
for e in EV:
    t, k = e['t'], e['type']
    if k == 'key':   # 키보드: 썸 + 클릭, 키마다 음색 미세 변화
        g = .8 + .4 * rng.random(); add(fx, sfx.thump(.05, 110 + 30 * rng.random()) * .5 * g, t); add(fx, bp(noise(.025), 2500, 7000) * env_exp(.025, .005) * .16 * g, t, 1, (rng.random() - .5) * .3)
    elif k == 'click': add(fx, sfx.click(1.4, .35), t); add(fx, lp(noise(.03), 2000) * env_exp(.03, .006) * .2, t)
    elif k == 'snap': add(fx, sfx.pop(.18), t); add(fx, square(hz(84), .08, .05, .5), t + .03, 1, .2)
    elif k == 'drag': d = e['d']; add(fx, bp(noise(d), 600, 2400) * np.sin(np.pi * t_(d) / d) * .05, t)
    elif k == 'open': add(fx, sfx.whoosh(.3, .14), t, 1, .2); add(fx, sfx.pop(.1), t + .2)
    elif k == 'whoosh': add(fx, sfx.whoosh(e.get('d', .4), .25), t)
    elif k == 'saved': add(fx, sfx.ding(.22), t)
    elif k == 'check': add(fx, square(hz(79 + 5 * e['n']), .09, .06, .25), t, 1, .2)
    elif k == 'done':
        add(fx, sfx.ding(.3), t)
        for i, m in enumerate([84, 88, 91, 96]): add(fx, square(hz(m), .1, .06, .5), t + .05 + i * .06, 1, .3)
    elif k == 'tick': add(fx, square(hz(84), .07, .05, .25), t)
    elif k == 'pop': add(fx, sfx.pop(.14), t)
    elif k == 'wobble': add(fx, square(330, .5, .04, .5, slide=.3) * (1 + .5 * np.sin(2 * np.pi * 14 * t_(.5))), t)
    elif k == 'jump': add(fx, square(440, .1, .06, .5, slide=1.2), t, 1, .1)
    elif k == 'land': add(fx, square(220, .07, .06, .5, slide=-.5), t); add(fx, sfx.thump(.12, 90), t)
    elif k == 'bud': add(fx, square(hz(SCALE[e['i'] % len(SCALE)]), .09, .045, .5), t, 1, (e['i'] % 5 - 2) * .15)
    elif k == 'slap': add(fx, lp(noise(.09), 3500) * env_exp(.09, .02) * .5, t); add(fx, sfx.thump(.1, 80) * .6, t)
    elif k == 'squish': add(fx, square(500, .3, .06, .5, slide=-.7), t)
    elif k == 'shuffle': d = e['d']; add(fx, bp(noise(d), 900, 4000) * np.sin(np.pi * t_(d) / d) * .07, t)
    elif k == 'whee': add(fx, square(400, .25, .05, .5, slide=1.0), t, 1, .2)
    elif k == 'miss': add(fx, square(180, .12, .05, .5), t); add(fx, sfx.click(.8, .2), t)
    elif k == 'bang': add(fx, square(hz(88), .06, .06, .5), t); add(fx, square(hz(93), .1, .06, .5), t + .07)
    elif k == 'rally': add(fx, square(330, .4, .06, .5, slide=1.5), t)
    elif k == 'found':
        add(fx, sfx.ding(.3), t)
        for i, m in enumerate([79, 84, 88]): add(fx, square(hz(m), .08, .05, .5), t + .04 + i * .05, 1, .3)
    elif k == 'bird':
        for i in range(4): add(fx, square(hz(96 + (i % 2) * 3), .05, .025, .5), t + i * .06, 1, .4)
    elif k == 'wipe':
        for i in range(10): add(fx, square(hz(60 + i * 2), .04, .03, .5), t + i * .045, 1, -.4 + i * .08)

# ── 앰비언스: 방 톤, 한 번도 0이 되지 않는다 ──
bed = lp(brown(DUR), 400); bed = bed / np.abs(bed).max() * .018
amb[:, 0] = bed; amb[:, 1] = np.roll(bed, 2400)

# ── 믹스: 효과음 아래로 음악 덕킹, 리미트 ──
env = np.ones(N)
for e in EV:
    if e['type'] in ('found', 'done', 'click', 'snap', 'slap'):
        a = int(e['t'] * SR); env[a:a + int(.25 * SR)] = np.minimum(env[a:a + int(.25 * SR)], .6)
env = lp(env, 8)
mix = music * env[:, None] * .7 + fx + amb
mix = mix / max(1e-9, np.abs(mix).max()) * .9
mix = np.stack([sfx.limit(mix[:, c]) for c in (0, 1)], 1)
(HERE / 'out').mkdir(exist_ok=True)
write(HERE / 'out/mix.wav', SR, (np.clip(mix, -1, 1) * 32767).astype(np.int16))
print('mix ok', DUR)

# 자막(번인과 같은 시각) → exports/subtitles_ko.srt
ts = lambda x: f"{int(x//3600):02}:{int(x//60%60):02}:{int(x%60):02},{round(x%1*1000):03}"
subs = [e for e in EV if e['type'] == 'sub']
(HERE.parent / 'exports').mkdir(exist_ok=True)
(HERE.parent / 'exports/subtitles_ko.srt').write_text('\n\n'.join(f"{i+1}\n{ts(e['t'])} --> {ts(e['t1'])}\n{e['text']}" for i, e in enumerate(subs)) + '\n')
