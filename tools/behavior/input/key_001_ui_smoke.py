from tools.behavior.memoivy_driver import (call, wait_window, key, clip, binary, environment, write_notes)
# 검증: KEY-001, FMT-002, SAVE-001. 합성 메모만 사용하며 xdotool·xclip·Pillow가 필요하다.
import json
import os
from pathlib import Path
import subprocess as sp
import tempfile
import time
import uuid

artifacts = Path(os.environ.get('MEMOIVY_ARTIFACT_DIR', '/tmp/memoivy-keyboard'))
artifacts.mkdir(parents=True, exist_ok=True)
original = '기존 메모'


def copied_source():
    key('ctrl+a')
    key('ctrl+c')
    return sp.check_output([clip, '-selection', 'clipboard', '-o'], text=True, timeout=5)


with tempfile.TemporaryDirectory(prefix='memoivy-live-') as tmp:
    env = environment(tmp)
    path = write_notes(tmp, [dict(id=str(uuid.uuid4()), text=original, color='yellow', modified=1)])
    log = open(artifacts / 'app.log', 'w')
    wm = sp.Popen([os.environ.get('MEMOIVY_OPENBOX', 'openbox'), '--sm-disable'], env=env, stdout=log, stderr=log)
    app = None
    try:
        time.sleep(.6)
        assert wm.poll() is None
        app = sp.Popen([binary], env=env, stdout=log, stderr=log)
        library = wait_window('^MemoIvy$')
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
