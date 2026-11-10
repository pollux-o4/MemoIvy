"""NOTE-004: preview disclosure with real pointer/keyboard and unchanged source."""
import json
import os
from pathlib import Path
import subprocess as sp
import tempfile
import time
import uuid
from PIL import ImageGrab, ImageChops
from tools.behavior.memoivy_driver import environment, write_notes, wait_window, call, click, key, binary

artifacts = Path(os.environ.get('MEMOIVY_ARTIFACT_DIR', '/tmp/memoivy-preview-e2e'))
artifacts.mkdir(parents=True, exist_ok=True)
with tempfile.TemporaryDirectory(prefix='memoivy-preview-') as tmp:
    env = environment(tmp)
    notes = [dict(id=str(uuid.uuid4()), text=text, color='yellow', modified=i,
                  canvas=dict(position=[64, 64], next=[], done=False))
             for i, text in enumerate([
                 '짧은 메모\n간단한 본문입니다.',
                 '프로젝트 진행 기록\n처음 두 줄만 보고도 메모의 내용을 파악할 수 있어요.\n더 보기를 누르면 목록 안에서 본문이 펼쳐집니다.\n\n**검토 항목**\n- 검색 결과에서도 같은 방식으로 읽기\n- 좁은 창에서는 자연스럽게 줄바꿈\n- 본문을 읽은 뒤 접어서 목록으로 돌아오기\n\n마지막 문장까지 잘 보이는지 확인합니다.',
             ])]
    path = write_notes(tmp, notes)
    original = path.read_bytes()
    log = open(artifacts / 'app.log', 'w')
    wm = sp.Popen([os.environ.get('MEMOIVY_OPENBOX', 'openbox'), '--sm-disable'], env=env, stdout=log, stderr=log)
    app = None
    def image(window, name):
        call('mousemove', '900', '700')
        time.sleep(.4)
        geom = dict(line.split('=', 1) for line in call('getwindowgeometry', '--shell', window).splitlines())
        x, y, w, h = [int(geom[k]) for k in ('X', 'Y', 'WIDTH', 'HEIGHT')]
        shot = ImageGrab.grab().crop((x, y, x+w, y+h))
        shot.save(artifacts / (name + '.png'))
        return shot
    def second_card_y(shot):
        starts = []
        previous = False
        for py in range(145, shot.height):
            # Window-manager frame insets vary; find the accent within the card's left gutter.
            stripe = any(r > 150 and g > 110 and r > b + 50 and g > b + 30
                         for r, g, b in (shot.getpixel((px, py))[:3] for px in range(20, 48)))
            if stripe and not previous:
                starts.append(py)
            previous = stripe
        assert len(starts) == 2, f'expected two cards, got {starts}'
        return starts[1]
    try:
        time.sleep(.4)
        app = sp.Popen([binary], env=env, stdout=log, stderr=log)
        window = wait_window('^MemoIvy$')
        call('windowmove', window, '0', '0')
        call('windowfocus', window)
        initial = image(window, '01-collapsed')
        click(window, 220, 248)
        expanded = image(window, '02-expanded')
        assert not sp.run([os.environ.get('MEMOIVY_XDOTOOL', 'xdotool'), 'search', '--onlyvisible', '--name', '^메모$'], capture_output=True).stdout, 'reveal must not open editor'
        assert ImageChops.difference(initial, expanded).getbbox(), 'disclosure must change rendered card'
        assert second_card_y(expanded) > second_card_y(initial) + 80, 'expanded body must move following card down'
        key('space')
        collapsed = image(window, '03-keyboard-collapsed')
        assert second_card_y(collapsed) == second_card_y(initial), 'Space must collapse the same card'
        # Repeated keyboard activation stays on the same control.
        key('Return')
        assert second_card_y(image(window, '04-keyboard-expanded')) == second_card_y(expanded), 'Enter must expand the same card'
        call('windowsize', window, '360', '700')
        image(window, '05-narrow-expanded')
        key('space')
        image(window, '06-narrow-collapsed')
        click(window, 100, 75)
        call('type', '--clearmodifiers', '진행')
        image(window, '07-search-collapsed')
        click(window, 175, 248)
        image(window, '08-search-expanded')
        assert not sp.run([os.environ.get('MEMOIVY_XDOTOOL', 'xdotool'), 'search', '--onlyvisible', '--name', '^메모$'], capture_output=True).stdout, 'search disclosure must not open editor'
        click(window, 100, 75)
        key('ctrl+a'); key('BackSpace')
        image(window, '09-search-cleared')
        # Title still opens the existing editor.
        click(window, 145, 179)
        editor = wait_window('^메모$')
        image(editor, '10-editor')
        click(editor, 34, 26)
        click(window, 34, 26)
        app.wait(timeout=6)
        assert json.loads(path.read_bytes()) == json.loads(original), 'preview must preserve all source data'
        assert 'Gtk-WARNING' not in (artifacts / 'app.log').read_text(), 'GTK layout warnings'
        print('PASS: pointer/Space/Enter, narrow/search preview, separate editor action, source preservation')
        print('Screenshots:', artifacts)
    finally:
        if app and app.poll() is None:
            app.terminate(); app.wait(timeout=5)
        wm.terminate(); wm.wait(timeout=5)
        log.close()
