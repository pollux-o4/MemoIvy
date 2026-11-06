from tools.behavior.memoivy_driver import (call, wait_window, key, click, move, clip, binary, environment, write_notes, uri_handler, CLOSE, LIBRARY_NOTE)
# 검증: FMT-001, SAVE-001. 합성 메모만 사용하며 xdotool·xclip·Pillow가 필요하다.
import json
import os
from pathlib import Path
import subprocess as sp
import tempfile
import time
import uuid
from PIL import ImageGrab

artifacts = Path(os.environ.get('MEMOIVY_ARTIFACT_DIR', '/tmp/memoivy-live-preview'))
artifacts.mkdir(parents=True, exist_ok=True)
original = '# 실시간 메모 📝\n## 오늘 할 일\n**굵게** · *기울임* · ~~취소~~\n- 글머리표\n1. 번호 목록\n- [ ] 테스트 할 일\n- [x] 완료한 일\n[예시 링크](https://example.com)\n\n끝'


MORE_MENU = (465, 26)
SOURCE_TOGGLE = (55, 258)
TASK_CHECKBOX = (36, 212)
SELECTION_START = (25, 143)
SELECTION_END = (170, 143)

def screenshot(note, name):
    geometry = dict(line.split('=', 1) for line in call('getwindowgeometry', '--shell', note).splitlines())
    px, py, width, height = [int(geometry[k]) for k in ('X', 'Y', 'WIDTH', 'HEIGHT')]
    ImageGrab.grab(xdisplay=os.environ['DISPLAY']).crop((px, py, px+width, py+height)).save(artifacts / name)


def copied_source():
    key('ctrl+a')
    key('ctrl+c')
    return sp.check_output([clip, '-selection', 'clipboard', '-o'], text=True, timeout=5)


with tempfile.TemporaryDirectory(prefix='memoivy-live-') as tmp:
    env = environment(tmp)
    path = write_notes(tmp, [dict(id=str(uuid.uuid4()), text=original, color='yellow', modified=1)])
    # Isolated URI handler records the URL instead of opening an external browser.
    uri_log = uri_handler(tmp, env)
    log = open(artifacts / 'app.log', 'w')
    wm = sp.Popen([os.environ.get('MEMOIVY_OPENBOX', 'openbox'), '--sm-disable'], env=env, stdout=log, stderr=log)
    app = None
    try:
        time.sleep(.6)
        assert wm.poll() is None
        app = sp.Popen([binary], env=env, stdout=log, stderr=log)
        library = wait_window('^MemoIvy$')
        call('windowfocus', library)
        click(library, *LIBRARY_NOTE)
        note = wait_window('^메모$')
        call('windowsize', note, '500', '570')
        call('windowraise', note)
        call('windowfocus', note)
        key('ctrl+End')
        screenshot(note, 'live.png')
        click(note, *MORE_MENU)
        screenshot(note, 'more-menu.png')
        key('Escape')
        click(note, *SOURCE_TOGGLE)
        assert not uri_log.exists(), 'ordinary link click must keep editing'
        key('ctrl+End')
        call('keydown', 'ctrl')
        click(note, *SOURCE_TOGGLE)
        call('keyup', 'ctrl')
        for _ in range(60):
            if uri_log.exists(): break
            time.sleep(.05)
        assert uri_log.exists() and uri_log.read_text() == 'https://example.com', 'Ctrl+click URI handler'
        key('ctrl+Home')
        key('ctrl+Home')
        screenshot(note, 'active-heading.png')
        key('End', 'BackSpace')
        assert copied_source() == original.replace(' 📝', ' '), 'heading end Backspace'
        key('Right', 'ctrl+z')
        assert copied_source() == original, 'undo preserves Markdown'
        key('Right', 'ctrl+Home', 'End', 'Delete')
        assert copied_source() == original.replace('📝\n##', '📝##'), 'newline deletion'
        key('Right', 'ctrl+z')
        assert copied_source() == original
        key('Right', 'ctrl+Home')
        # Enter the task line by source line navigation, then keyboard toggle and single undo.
        key('Down', 'Down', 'Down', 'Down', 'Down', 'Home', 'ctrl+Return')
        assert copied_source() == original.replace('[ ]', '[x]', 1), 'keyboard task toggle'
        key('Right', 'ctrl+z')
        assert copied_source() == original, 'one undo restores task state'
        key('Right', 'ctrl+End')
        screenshot(note, 'live-final.png')
        move(note, *SELECTION_START)
        call('mousedown', '1')
        call('mousemove', '--sync', '--window', note, *map(str, SELECTION_END))
        call('mouseup', '1')
        time.sleep(.2)
        screenshot(note, 'drag-selection.png')
        assert copied_source() == original, 'mouse selection must not change raw content'
        key('Right', 'ctrl+End')
        click(note, *TASK_CHECKBOX)
        assert copied_source() == original.replace('[ ]', '[x]', 1), 'native checkbox mouse toggle'
        key('Right', 'ctrl+z')
        assert copied_source() == original
        key('Right', 'ctrl+End')
        call('windowsize', note, '330', '570')
        time.sleep(.3)
        screenshot(note, 'narrow.png')
        click(note, *TASK_CHECKBOX)
        assert copied_source() == original.replace('[ ]', '[x]', 1), 'checkbox stays aligned after resize'
        key('Right', 'ctrl+z')
        key('ctrl+End')
        call('windowsize', note, '500', '570')
        time.sleep(.3)
        # Raw mode preserves source and selection; switching back keeps the same editor.
        key('ctrl+shift+m')
        screenshot(note, 'source.png')
        assert copied_source() == original
        key('ctrl+shift+m')
        key('ctrl+End')
        assert copied_source() == original
        key('Right', 'ctrl+End')
        key('Return')
        call('type', '--clearmodifiers', '--', '**live**')
        key('Return')
        screenshot(note, 'live-typed.png')
        assert copied_source() == original + '\n**live**\n', 'new Markdown is editable source'
        key('Right', 'ctrl+End')
        call('type', '--clearmodifiers', '--', ' saved')
        click(note, *CLOSE)
        expected = original + '\n**live**\n saved'
        for _ in range(120):
            if json.loads(path.read_text())[0]['text'] == expected:
                break
            time.sleep(.05)
        else:
            raise AssertionError('SAVE-001 lost Markdown: ' + path.read_text())
        call('windowfocus', library)
        click(library, *CLOSE)
        assert app.wait(timeout=8) == 0
        app = sp.Popen([binary], env=env, stdout=log, stderr=log)
        library = wait_window('^MemoIvy$')
        call('windowfocus', library)
        click(library, *LIBRARY_NOTE)
        note = wait_window('^메모$')
        call('windowfocus', note)
        assert copied_source() == expected, 'reopened editor restores complete raw Markdown'
        log.flush()
        assert 'panicked' not in (artifacts / 'app.log').read_text()
        assert 'Gtk-CRITICAL' not in (artifacts / 'app.log').read_text()
        print('PASS: live editing, Unicode, selection/copy, delete boundaries, undo/redo, task mouse/keyboard, resize, safe link gesture, source mode, close/reopen')
        print('screenshots:', artifacts)
    finally:
        if app and app.poll() is None:
            app.terminate()
            app.wait(timeout=5)
        wm.terminate()
        wm.wait(timeout=5)
        log.close()
