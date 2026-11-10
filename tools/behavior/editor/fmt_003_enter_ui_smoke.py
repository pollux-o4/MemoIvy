"""FMT-003: Enter between nested items preserves following rendered paragraphs."""
import json
import os
from pathlib import Path
import subprocess as sp
import tempfile
import time
import uuid
from PIL import ImageGrab, ImageChops, ImageStat
from tools.behavior.memoivy_driver import call, wait_window, key, click, clip, binary, environment, write_notes

artifacts = Path(os.environ.get('MEMOIVY_ARTIFACT_DIR', '/tmp/memoivy-enter-e2e'))
artifacts.mkdir(parents=True, exist_ok=True)
original = '- 상위 목록 본문\n  - 중간 목록 본문\n    - 하위 목록 본문\n  - 블로그 (2) 작업\n- [x] 확인한 내용과 작업 상태'
expected = original.replace('    - 하위 목록 본문\n', '    - 하위 목록 본문\n    - \n')


def copied():
    key('ctrl+a', 'ctrl+c')
    return sp.check_output([clip, '-selection', 'clipboard', '-o'], text=True, timeout=5)


def capture(name):
    time.sleep(.3)
    image = ImageGrab.grab(xdisplay=os.environ['DISPLAY']).crop((500, 60, 1120, 580)).convert('RGB')
    image.save(artifacts / (name + '.png'))
    return image


def text_rows(image):
    background = image.getpixel((595, 100))
    rows = []
    for y in range(65, 300):
        ink = any(max(abs(c-b) for c,b in zip(image.getpixel((x,y)), background)) > 55
                  and max(image.getpixel((x,y))) - min(image.getpixel((x,y))) < 45
                  for x in range(110, 400))
        if ink:
            if rows and y <= rows[-1][-1] + 3:
                rows[-1].append(y)
            else:
                rows.append([y])
    return [row for row in rows if len(row) >= 4]


with tempfile.TemporaryDirectory(prefix='memoivy-enter-') as tmp:
    env = environment(tmp)
    path = write_notes(tmp, [dict(id=str(uuid.uuid4()), text=original, color='yellow', modified=1, canvas=dict(position=[64,64], next=[], done=False))])
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
            key('ctrl+Home', 'Down', 'Down', 'End')
            before = capture('before')
            rows = text_rows(before)
            assert len(rows) == 5, f'expected five text rows: {rows}'
            blog = before.crop((20, rows[3][0], 400, rows[3][-1]+1))
            key('Return')
            after = capture('after-enter')
            after_rows = text_rows(after)
            assert len(after_rows) == 5, f'empty item must add no text: {after_rows}'
            actual = after.crop((20, after_rows[3][0], 400, after_rows[3][0]+blog.height))
            difference = ImageStat.Stat(ImageChops.difference(blog, actual)).mean
            assert max(difference) < 1.0, f'Enter changed following bullet/body pixels: {difference}'
            line_height = rows[2][0] - rows[1][0]
            assert abs(after_rows[3][0] - rows[3][0] - line_height) <= 1, 'Enter must reserve exactly one line'
            assert copied() == expected, 'Enter source differs'
            key('Right', 'ctrl+z')
            assert copied() == original, 'one undo must restore source'
            key('Right', 'ctrl+shift+z')
            assert copied() == expected, 'redo must restore inserted item'
            key('Right')
            time.sleep(.8)
            assert json.loads(path.read_text())[0]['text'] == expected, 'saved source differs'
            key('ctrl+w')
            call('windowfocus', library)
            click(library, 150, 175)
            note = wait_window('^메모$')
            assert copied() == expected, 'reopened source differs'
            print('PASS: actual Enter, following bullet/body pixels, clipboard, undo/redo, save/reopen')
        finally:
            if app and app.poll() is None:
                app.terminate()
                app.wait(timeout=5)
            wm.terminate()
            wm.wait(timeout=5)
