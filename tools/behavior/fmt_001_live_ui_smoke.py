# 검증: FMT-001, SAVE-001. 합성 메모만 사용하며 xdotool·xclip·Pillow가 필요하다.
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
    # Isolated URI handler records the URL instead of opening an external browser.
    apps = Path(tmp) / 'applications'
    apps.mkdir()
    capture = Path(tmp) / 'capture_uri.py'
    uri_log = Path(tmp) / 'uri.txt'
    capture.write_text('import sys\nfrom pathlib import Path\nPath(' + repr(str(uri_log)) + ').write_text(sys.argv[1])\n')
    (apps / 'lsm-uri-test.desktop').write_text('[Desktop Entry]\nType=Application\nName=LSM URI test\nExec=/usr/bin/python3 ' + str(capture) + ' %u\nMimeType=x-scheme-handler/https;\n')
    Path(env['XDG_CONFIG_HOME']).mkdir()
    (Path(env['XDG_CONFIG_HOME']) / 'mimeapps.list').write_text('[Default Applications]\nx-scheme-handler/https=lsm-uri-test.desktop;\n')
    log = open(artifacts / 'app.log', 'w')
    wm = sp.Popen([os.environ.get('LSM_OPENBOX', 'openbox'), '--sm-disable'], env=env, stdout=log, stderr=log)
    app = None
    try:
        time.sleep(.6)
        assert wm.poll() is None
        app = sp.Popen([binary], env=env, stdout=log, stderr=log)
        library = wait_window('^스티키 메모$')
        call('windowfocus', library)
        click(library, 150, 175)
        note = wait_window('^메모$')
        call('windowsize', note, '500', '570')
        call('windowraise', note)
        call('windowfocus', note)
        key('ctrl+End')
        screenshot(note, 'live.png')
        click(note, 465, 26)
        screenshot(note, 'more-menu.png')
        key('Escape')
        click(note, 55, 258)
        assert not uri_log.exists(), 'ordinary link click must keep editing'
        key('ctrl+End')
        call('keydown', 'ctrl')
        click(note, 55, 258)
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
        call('mousemove', '--window', note, '25', '143')
        call('mousedown', '1')
        call('mousemove', '--sync', '--window', note, '170', '143')
        call('mouseup', '1')
        time.sleep(.2)
        screenshot(note, 'drag-selection.png')
        assert copied_source() == original, 'mouse selection must not change raw content'
        key('Right', 'ctrl+End')
        click(note, 36, 212)
        assert copied_source() == original.replace('[ ]', '[x]', 1), 'native checkbox mouse toggle'
        key('Right', 'ctrl+z')
        assert copied_source() == original
        key('Right', 'ctrl+End')
        call('windowsize', note, '330', '570')
        time.sleep(.3)
        screenshot(note, 'narrow.png')
        click(note, 36, 212)
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
        click(note, 34, 26)
        expected = original + '\n**live**\n saved'
        for _ in range(120):
            if json.loads(path.read_text())[0]['text'] == expected:
                break
            time.sleep(.05)
        else:
            raise AssertionError('SAVE-001 lost Markdown: ' + path.read_text())
        call('windowfocus', library)
        click(library, 34, 26)
        assert app.wait(timeout=8) == 0
        app = sp.Popen([binary], env=env, stdout=log, stderr=log)
        library = wait_window('^스티키 메모$')
        call('windowfocus', library)
        click(library, 150, 175)
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
