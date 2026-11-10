"""FMT-004: real pointer routing to anchored image controls."""
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess as sp
import tempfile
import time
import uuid
from PIL import Image, ImageGrab
from tools.behavior.memoivy_driver import call, wait_window, key, click, move, clip, binary, environment, write_notes

artifacts = Path(os.environ.get('MEMOIVY_ARTIFACT_DIR', '/tmp/memoivy-image-e2e'))
artifacts.mkdir(parents=True, exist_ok=True)
with tempfile.TemporaryDirectory(prefix='memoivy-image-') as tmp:
    env = environment(tmp)
    output = io.BytesIO()
    Image.new('RGB', (400, 200), (70, 140, 210)).save(output, format='PNG')
    data = output.getvalue()
    asset = hashlib.sha256(data).hexdigest()
    original = f'- [x] 마일스톤 확인했음\n![클릭 회귀|340](asset:{asset})\n아래 본문'
    path = write_notes(tmp, [dict(id=str(uuid.uuid4()), text=original, color='yellow', modified=1)])
    assets = path.parent / 'assets'
    assets.mkdir()
    (assets / (asset + '.png')).write_bytes(data)
    with open(artifacts / 'app.log', 'w') as log:
        wm = sp.Popen([os.environ.get('MEMOIVY_OPENBOX', 'openbox'), '--sm-disable'], env=env, stdout=log, stderr=log)
        app = None
        try:
            time.sleep(.5)
            app = sp.Popen([binary], env=env, stdout=log, stderr=log)
            library = wait_window('^MemoIvy$')
            call('windowmove', library, '0', '0')
            click(library, 150, 175)
            note = wait_window('^메모$')
            call('windowmove', note, '500', '60')
            call('windowsize', note, '620', '520')
            call('windowfocus', note)
            time.sleep(.6)
            move(note, 180, 160)
            time.sleep(.3)
            ImageGrab.grab(xdisplay=os.environ['DISPLAY']).save(artifacts / 'hover.png')
            call('click', '--repeat', '2', '--delay', '100', '1')
            viewer = wait_window('클릭 회귀')
            ImageGrab.grab(xdisplay=os.environ['DISPLAY']).save(artifacts / 'viewer.png')
            key('Escape')
            time.sleep(.3)
            assert json.loads(path.read_text())[0]['text'] == original, 'viewing must preserve source'
            def copied():
                key('ctrl+a', 'ctrl+c')
                result = sp.check_output([clip, '-selection', 'clipboard', '-o'], text=True, timeout=5)
                key('Right')
                return result
            assert sp.run([os.environ.get('MEMOIVY_XDOTOOL', 'xdotool'), 'search', '--onlyvisible', '--name', '^클릭 회귀$'], capture_output=True).returncode != 0, 'Escape must close viewer'
            click(note, 315, 114)
            wait_window('^클릭 회귀$')
            key('Escape')
            time.sleep(.3)
            # A real corner drag must update width once and undo as a single action.
            move(note, 352, 256)
            call('mousedown', '1')
            call('mousemove', '--window', note, '412', '286')
            call('mouseup', '1')
            time.sleep(.3)
            resized = copied()
            assert resized != original and f'(asset:{asset})' in resized, 'corner drag must resize source'
            key('ctrl+z')
            assert copied() == original, 'resize undo must restore exact source'
            move(note, 180, 160)
            time.sleep(.2)
            click(note, 343, 114)
            assert 'asset:' not in copied(), 'trash control must remove the reference'
            assert (assets / (asset + '.png')).exists(), 'remove must preserve stored bytes'
            key('ctrl+z')
            assert copied() == original, 'remove undo must restore exact source'
            ImageGrab.grab(xdisplay=os.environ['DISPLAY']).save(artifacts / 'restored.png')

            print('PASS: hover, double click, expand button, Escape, resize/remove and single undo, source/asset preservation')
        finally:
            if app and app.poll() is None:
                app.terminate()
                app.wait(timeout=5)
            wm.terminate()
            wm.wait(timeout=5)
