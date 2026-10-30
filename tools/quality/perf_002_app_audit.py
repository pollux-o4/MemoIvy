#!/usr/bin/env python3
# 검증: PERF-002. 실제 릴리스 실행 파일의 시작·메모리·유휴·반응과 색 대비를 합성 데이터로 측정한다.
# 사용: dbus-run-session -- xvfb-run -a -s '-screen 0 1400x900x24' python3 tools/quality/perf_002_app_audit.py
import json, os, re, statistics, subprocess as sp, tempfile, time, uuid
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
x = os.environ.get('LSM_XDOTOOL', 'xdotool')
binary = os.environ.get('LSM_BINARY', str(ROOT / 'target/release/linux-sticky-memo'))
NOTES = int(os.environ.get('LSM_AUDIT_NOTES', '300'))


def call(*args):
    return sp.check_output([x, *args], text=True, timeout=10).strip()


def wait_window(title):
    for _ in range(2000):
        r = sp.run([x, 'search', '--onlyvisible', '--name', title], capture_output=True, text=True)
        if r.returncode == 0:
            return r.stdout.splitlines()[-1]
        time.sleep(.005)
    raise SystemExit('window missing: ' + title)


def seed(path):
    ids = [str(uuid.uuid4()) for _ in range(NOTES)]
    colors = ['yellow', 'pink', 'blue', 'green', 'purple']
    path.parent.mkdir(parents=True)
    path.write_text(json.dumps([
        dict(id=i, text=f'# 메모 {n}\n- [x] 완료\n- [ ] 할 일\n' + '본문 내용 ' * 90, color=colors[n % 5], modified=1,
             canvas=dict(position=[(n % 20) * 300, (n // 20) * 200], next=[ids[n + 1]] if n < NOTES - 1 and n % 3 else [], done=False))
        for n, i in enumerate(ids)], ensure_ascii=False))


def cpu(pid):
    f = Path(f'/proc/{pid}/stat').read_text().split()
    return (int(f[13]) + int(f[14])) / os.sysconf('SC_CLK_TCK')


def rss_mb(pid):
    return int(re.search(r'VmRSS:\s+(\d+)', Path(f'/proc/{pid}/status').read_text()).group(1)) / 1024


def run_once():
    tmp = tempfile.mkdtemp(prefix='lsm-audit-')
    env = dict(os.environ, GDK_BACKEND='x11', GDK_DEBUG='no-portals', GTK_USE_PORTAL='0', GIO_USE_VFS='local', GTK_A11Y='none', XDG_DATA_HOME=tmp, XDG_CONFIG_HOME=tmp + '/c')
    seed(Path(tmp) / 'linux-sticky-memo/notes.json')
    wm = sp.Popen([os.environ.get('LSM_OPENBOX', 'openbox'), '--sm-disable'], env=env, stdout=sp.DEVNULL, stderr=sp.DEVNULL)
    time.sleep(.5)
    begin = time.perf_counter()
    app = sp.Popen([binary], env=env, stdout=sp.DEVNULL, stderr=sp.DEVNULL)
    try:
        library = wait_window('^스티키 메모$')
        started = time.perf_counter() - begin
        call('windowmove', library, '0', '0'); call('windowfocus', library); time.sleep(.3)
        call('mousemove', '--window', library, '300', '24')
        begin = time.perf_counter(); call('click', '1')
        canvas = wait_window('^메모 캔버스$')
        opened = time.perf_counter() - begin
        call('windowmove', canvas, '0', '0'); call('windowfocus', canvas); time.sleep(1)
        rss = rss_mb(app.pid)
        a = cpu(app.pid); time.sleep(2); idle = cpu(app.pid) - a
        call('mousemove', '--window', canvas, '600', '400')
        a = cpu(app.pid); t = time.perf_counter()
        for _ in range(20):
            call('click', '5'); time.sleep(.04)
        pan = (cpu(app.pid) - a) / (time.perf_counter() - t)
        time.sleep(1)
        a = cpu(app.pid); time.sleep(2); after = cpu(app.pid) - a
        return dict(start_s=started, canvas_open_s=opened, rss_mb=rss, idle_cpu_pct=idle / 2 * 100, pan_cpu_pct=pan * 100, idle_after_pct=after / 2 * 100)
    finally:
        app.terminate()
        try:
            app.wait(timeout=5)
        except sp.TimeoutExpired:
            app.kill()
        wm.terminate(); wm.wait(timeout=5)


def tokens(name):
    out = {}
    for key, value in re.findall(r'@define-color (\w+) ([^;]+);', (ROOT / f'src/presentation/styles/tokens-{name}.css').read_text()):
        m = re.match(r'alpha\((#\w+), (\.?\d+)\)', value)
        out[key] = (m.group(1), float(m.group(2))) if m else (value, 1.)
    return out


def rgb(h):
    return [int(h[i:i + 2], 16) / 255 for i in (1, 3, 5)]


def over(fg, alpha, bg):
    return [f * alpha + b * (1 - alpha) for f, b in zip(fg, bg)]


def lum(c):
    v = [x / 12.92 if x <= .03928 else ((x + .055) / 1.055) ** 2.4 for x in c]
    return .2126 * v[0] + .7152 * v[1] + .0722 * v[2]


def ratio(a, b):
    la, lb = sorted((lum(a), lum(b)), reverse=True)
    return (la + .05) / (lb + .05)


def contrast():
    failures = []
    for theme in ('light', 'dark'):
        t = tokens(theme)
        solid = lambda k: rgb(t[k][0])
        pairs = [('lsm_text', 'lsm_bg'), ('lsm_muted', 'lsm_bg'), ('lsm_text', 'lsm_raised'), ('lsm_muted', 'lsm_raised'), ('lsm_on_accent', 'lsm_accent'), ('lsm_danger', 'lsm_raised')]
        for fg, bg in pairs:
            r = ratio(over(rgb(t[fg][0]), t[fg][1], solid(bg)), solid(bg))
            if r < 4.5:
                failures.append((theme, fg, bg, round(r, 2)))
        r = ratio([1, 1, 1], solid('lsm_danger_fill'))
        if r < 4.5:
            failures.append((theme, 'white', 'lsm_danger_fill', round(r, 2)))
        for note in ('yellow', 'pink', 'blue', 'green', 'purple'):
            bg = solid('note_' + note)
            for fg, floor in (('note_text', 4.5), ('note_muted', 4.5)):
                r = ratio(over(rgb(t[fg][0]), t[fg][1], bg), bg)
                if r < floor:
                    failures.append((theme, fg, note, round(r, 2)))
            # 완료 카드는 전체 불투명도를 낮춘다. 제목 대비를 보드 배경 위에서 다시 계산한다.
            faded_bg = over(bg, DONE_OPACITY, solid('lsm_bg'))
            faded = over(rgb(t['note_text'][0]), DONE_OPACITY, solid('lsm_bg'))
            r = ratio(faded, faded_bg)
            if r < 4.5:
                failures.append((theme, 'done-title', note, round(r, 2)))
            faded = over(over(rgb(t['note_muted'][0]), t['note_muted'][1], bg), DONE_OPACITY, solid('lsm_bg'))
            r = ratio(faded, faded_bg)
            if r < 3:
                failures.append((theme, 'done-preview', note, round(r, 2)))
    return failures


m = re.search(r'\.canvas-card\.done \{ opacity: (\.?\d+)', (ROOT / 'src/presentation/styles/canvas.css').read_text())
DONE_OPACITY = float(m.group(1)) if m else 1.
runs = [run_once() for _ in range(3)]
print(f'notes={NOTES} runs={len(runs)} (median)')
for key in runs[0]:
    print(f'  {key}: {statistics.median(r[key] for r in runs):.3f}')
failures = contrast()
print('contrast failures:', failures or 'none')
