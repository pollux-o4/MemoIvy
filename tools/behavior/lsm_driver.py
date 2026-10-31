"""Shared X11 smoke input and isolated fixture setup."""
import json
import os
from pathlib import Path
import subprocess as sp
import time

x = os.environ.get('LSM_XDOTOOL', 'xdotool')
clip = os.environ.get('LSM_XCLIP', 'xclip')
binary = os.environ.get('LSM_BINARY', 'target/debug/linux-sticky-memo')

# Window-relative controls shared by the smoke scenarios.
CLOSE = (34, 26)
LIBRARY_NOTE = (150, 175)
OPEN_CANVAS = (300, 24)
SEARCH = (100, 75)
NEW_NOTE = (385, 26)


def call(*args):
    return sp.check_output([x, *args], text=True, timeout=5).strip()


def wait_window(title):
    deadline = time.monotonic() + 6
    while time.monotonic() < deadline:
        result = sp.run([x, 'search', '--onlyvisible', '--name', title],
                        capture_output=True, text=True, timeout=5)
        if result.returncode == 0:
            return result.stdout.splitlines()[-1]
        time.sleep(.05)
    raise AssertionError('window missing: ' + title)


# shortcut: X11 입력 후 고정 대기로 UI 반영을 기다림, 스모크 실행 시간이 병목으로 확인되면 관찰 가능한 상태 대기로 교체.
def key(*keys):
    call('key', '--clearmodifiers', *keys)
    time.sleep(.15)


def move(window, px, py):
    call('mousemove', '--window', window, str(px), str(py))


def click(window, px, py):
    move(window, px, py)
    call('click', '1')
    time.sleep(.2)


def environment(tmp):
    return dict(os.environ, GDK_BACKEND='x11', GDK_DEBUG='no-portals', GSK_RENDERER='cairo',
                GTK_USE_PORTAL='0', GIO_USE_VFS='local', GTK_A11Y='none',
                XDG_CURRENT_DESKTOP='Openbox', XDG_DATA_HOME=str(tmp),
                XDG_CONFIG_HOME=str(Path(tmp) / 'config'))


def write_notes(tmp, notes):
    path = Path(tmp) / 'linux-sticky-memo/notes.json'
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(notes))
    return path


def uri_handler(tmp, env):
    apps = Path(tmp) / 'applications'
    apps.mkdir()
    capture = Path(tmp) / 'capture_uri.py'
    uri_log = Path(tmp) / 'uri.txt'
    capture.write_text('import sys\nfrom pathlib import Path\nPath(' + repr(str(uri_log)) + ').write_text(sys.argv[1])\n')
    # Desktop Exec quotes paths independently of shell quoting.
    executable = str(capture).replace('\\', '\\\\').replace('"', '\\"').replace('`', '\\`').replace('$', '\\$')
    (apps / 'lsm-uri-test.desktop').write_text('[Desktop Entry]\nType=Application\nName=LSM URI test\nExec=/usr/bin/python3 "' + executable + '" %u\nMimeType=x-scheme-handler/https;\n')
    config = Path(env['XDG_CONFIG_HOME'])
    config.mkdir(parents=True, exist_ok=True)
    (config / 'mimeapps.list').write_text('[Default Applications]\nx-scheme-handler/https=lsm-uri-test.desktop;\n')
    return uri_log
