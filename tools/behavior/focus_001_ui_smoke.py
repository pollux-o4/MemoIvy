# 검증: FOCUS-001. 합성 메모만 사용하며 xdotool·xclip·Pillow가 필요하다.
import json
import os
from pathlib import Path
import subprocess as sp
import tempfile
import time
import uuid
from PIL import ImageGrab

x = os.environ.get('LSM_XDOTOOL', 'xdotool')
clip = os.environ.get('LSM_XCLIP', 'xclip')
binary = os.environ.get('LSM_BINARY', 'target/debug/linux-sticky-memo')
artifacts = Path(os.environ.get('LSM_ARTIFACT_DIR', '/tmp/lsm-live-preview'))
artifacts.mkdir(parents=True, exist_ok=True)
original = '# 실시간 메모 📝\n## 오늘 할 일\n**굵게** · *기울임* · ~~취소~~\n- 글머리표\n1. 번호 목록\n- [ ] 테스트 할 일\n- [x] 완료한 일\n[예시 링크](https://example.com)\n\n끝'


def call(*args):
    return sp.check_output([x, *args], text=True, timeout=5).strip()


def wait_window(title):
    for _ in range(120):
        result = sp.run([x, 'search', '--onlyvisible', '--name', title], capture_output=True, text=True)
        if result.returncode == 0:
            return result.stdout.splitlines()[-1]
        time.sleep(.05)
    raise AssertionError('window missing: ' + title)


def key(*keys):
    call('key', '--clearmodifiers', *keys)
    time.sleep(.15)


def click(note, px, py):
    call('mousemove', '--window', note, str(px), str(py))
    call('click', '1')
    time.sleep(.2)


def screenshot(note, name):
    geometry = dict(line.split('=', 1) for line in call('getwindowgeometry', '--shell', note).splitlines())
    px, py, width, height = [int(geometry[k]) for k in ('X', 'Y', 'WIDTH', 'HEIGHT')]
    ImageGrab.grab(xdisplay=os.environ['DISPLAY']).crop((px, py, px+width, py+height)).save(artifacts / name)


def copied_source():
    key('ctrl+a')
    key('ctrl+c')
    return sp.check_output([clip, '-selection', 'clipboard', '-o'], text=True, timeout=5)


with tempfile.TemporaryDirectory(prefix='lsm-live-') as tmp:
    env = dict(os.environ, GDK_BACKEND='x11', GDK_DEBUG='no-portals', GSK_RENDERER='cairo',
               GTK_USE_PORTAL='0', XDG_CONFIG_HOME=tmp+'/config', GIO_USE_VFS='local', GTK_A11Y='none', XDG_CURRENT_DESKTOP='Openbox', XDG_DATA_HOME=tmp)
    path = Path(tmp) / 'linux-sticky-memo/notes.json'
    path.parent.mkdir()
    path.write_text(json.dumps([dict(id=str(uuid.uuid4()), text=original, color='yellow', modified=1)]))
    log = open(artifacts / 'app.log', 'w')
    wm = sp.Popen([os.environ.get('LSM_OPENBOX', 'openbox'), '--sm-disable'], env=env, stdout=log, stderr=log)
    app = None
    try:
        time.sleep(.6)
        assert wm.poll() is None
        app = sp.Popen([binary], env=env, stdout=log, stderr=log)
        library = wait_window('^스티키 메모$')
        call('windowmove', library, '10', '10')
        click(library, 150, 175)
        note = wait_window('^메모$')
        call('windowmove', note, '550', '10')
        for i in range(5):
            click(library, 100, 75)
            key('ctrl+a', 'BackSpace')
            call('type', '--clearmodifiers', '--', 'example')
            click(note, 75, 130)
            key('ctrl+End')
            call('type', '--clearmodifiers', '--', 'FOCUS')
            time.sleep(.5)
            assert json.loads(path.read_text())[0]['text'].endswith('FOCUS'*(i+1)), path.read_text()
            click(library, 100, 75)
            assert copied_source() == 'example', 'inactive search received memo input'
        print('PASS: mouse-only window focus transitions')
    finally:
        if app and app.poll() is None:
            app.terminate()
            app.wait(timeout=5)
        wm.terminate()
        wm.wait(timeout=5)
        log.close()
