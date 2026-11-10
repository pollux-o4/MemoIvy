"""FMT-003: actual window wrapping, checkbox activation and Markdown round trip."""
import json
import os
from pathlib import Path
import subprocess as sp
import tempfile
import time
import uuid
from PIL import ImageGrab
from tools.behavior.memoivy_driver import call, wait_window, key, click, clip, binary, environment, write_notes

artifacts = Path(os.environ.get('MEMOIVY_ARTIFACT_DIR', '/tmp/memoivy-wrap-e2e'))
artifacts.mkdir(parents=True, exist_ok=True)
original = '- [ ] ' + '가나다라마바사 ' * 24

def copied():
    key('ctrl+a', 'ctrl+c')
    return sp.check_output([clip, '-selection', 'clipboard', '-o'], text=True, timeout=5)

with tempfile.TemporaryDirectory(prefix='memoivy-wrap-') as tmp:
    env = environment(tmp)
    path = write_notes(tmp, [dict(id=str(uuid.uuid4()), text=original, color='yellow', modified=1, canvas=dict(position=[64,64], next=[], done=False))])
    log = open(artifacts / 'app.log', 'w')
    wm = sp.Popen([os.environ.get('MEMOIVY_OPENBOX', 'openbox'), '--sm-disable'], env=env, stdout=log, stderr=log)
    app = None
    try:
        time.sleep(.5)
        app = sp.Popen([binary], env=env, stdout=log, stderr=log)
        library = wait_window('^MemoIvy$')
        call('windowmove', library, '0', '0')
        time.sleep(.4)
        ImageGrab.grab(xdisplay=os.environ['DISPLAY']).save(artifacts / 'library.png')
        click(library, 150, 175)
        note = wait_window('^메모$')
        call('windowmove', note, '500', '60')
        call('windowfocus', note)
        for width in [380, 620, 380]:
            call('windowsize', note, str(width), '520')
            key('ctrl+Home')
            time.sleep(.5)
            image = ImageGrab.grab(xdisplay=os.environ['DISPLAY']).crop((500, 60, 500+width, 580)).convert('RGB')
            image.save(artifacts / f'wrap-{width}.png')
            background = image.getpixel((width-25, 100))
            rows = []
            for y in range(65, 390):
                xs = [x for x in range(48, width-25)
                      if max(abs(c-b) for c,b in zip(image.getpixel((x,y)), background)) > 55
                      and max(image.getpixel((x,y))) - min(image.getpixel((x,y))) < 45]
                if xs:
                    if rows and y == rows[-1][-1][0]+1:
                        rows[-1].append((y, min(xs)))
                    else:
                        rows.append([(y,min(xs))])
            columns = [min(x for _,x in row) for row in rows if len(row) >= 4]
            assert len(columns) >= 3, f'fixture must wrap: {columns}'
            assert 44 <= columns[0] <= 52, f'first line shifted: {columns}'
            assert max(columns)-min(columns) <= 1, f'wrapped text columns differ: {columns}'
            assert copied() == original, 'selection/copy must preserve source'
            key('ctrl+Home')
        click(note, 34, 85)
        expected = original.replace('[ ]','[x]',1)
        assert copied() == expected, 'visible checkbox must toggle exactly once'
        key('ctrl+End')
        call('type', '--delay', '20', 'typed')
        assert copied() == expected + 'typed'
        key('ctrl+End', 'ctrl+z')
        assert copied() == expected, 'one undo must restore typed text'
        key('ctrl+End')
        time.sleep(.8)
        assert json.loads(path.read_text())[0]['text'] == expected, 'saved source differs'
        key('ctrl+w')
        call('windowfocus', library)
        time.sleep(.4)
        ImageGrab.grab(xdisplay=os.environ['DISPLAY']).save(artifacts / 'library.png')
        click(library, 150, 175)
        note = wait_window('^메모$')
        assert copied() == expected, 'reopened note differs'
        print('PASS: narrow/wide wrapping, checkbox, typing/undo, clipboard, save/reopen')
    finally:
        if app and app.poll() is None:
            app.terminate()
            app.wait(timeout=5)
        wm.terminate()
        wm.wait(timeout=5)
