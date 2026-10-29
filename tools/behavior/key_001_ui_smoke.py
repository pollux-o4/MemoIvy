# 검증: KEY-001, FMT-002, SAVE-001. 합성 메모만 사용하며 xdotool·xclip·Pillow가 필요하다.
import json
import os
from pathlib import Path
import subprocess as sp
import tempfile
import time
import uuid

x = os.environ.get('LSM_XDOTOOL', 'xdotool')
clip = os.environ.get('LSM_XCLIP', 'xclip')
binary = os.environ.get('LSM_BINARY', 'target/debug/linux-sticky-memo')
artifacts = Path(os.environ.get('LSM_ARTIFACT_DIR', '/tmp/lsm-keyboard'))
artifacts.mkdir(parents=True, exist_ok=True)
original = '기존 메모'


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
        call('windowfocus', library)
        key('ctrl+n')
        note = wait_window('^메모$')
        call('type', '--clearmodifiers', '--', 'alpha')
        key('ctrl+a', 'ctrl+b')
        assert copied_source() == '**alpha**'
        key('Right', 'ctrl+a', 'ctrl+b')
        assert copied_source() == 'alpha'
        key('Right', 'ctrl+a', 'ctrl+i')
        assert copied_source() == '*alpha*'
        key('Right', 'ctrl+a', 'BackSpace')
        call('type', '--clearmodifiers', '--', 'Task')
        key('ctrl+shift+9', 'Right', 'ctrl+Return')
        assert copied_source() == '- [x] Task'
        key('Right', 'ctrl+w')
        time.sleep(.5)
        assert any(n['text'] == '- [x] Task' for n in json.loads(path.read_text()))
        key('ctrl+f')
        call('type', '--clearmodifiers', '--', 'Task')
        assert copied_source() == 'Task', 'Ctrl+F must focus search'
        key('ctrl+n')
        note = wait_window('^메모$')
        key('F1', 'Escape')
        key('ctrl+w')
        time.sleep(.4)
        key('ctrl+w')
        assert app.wait(timeout=8) == 0
        assert len(json.loads(path.read_text())) == 3
        print('PASS: new note, formatting toggle, checkbox, search, help, close/flush shortcuts')
    finally:
        if app and app.poll() is None:
            app.terminate()
            app.wait(timeout=5)
        wm.terminate()
        wm.wait(timeout=5)
        log.close()
