from tools.behavior.lsm_driver import (call, wait_window, key, click, move, clip, binary, environment, write_notes, uri_handler, OPEN_CANVAS)
# 검증: FMT-003, CANVAS-004. 합성 데이터로 링크 미리보기·목록 표식·캔버스 단축키를 실제 입력으로 확인한다.
import json
import os
from pathlib import Path
import subprocess as sp
import tempfile
import time
import uuid
from PIL import ImageChops, ImageGrab

artifacts = Path(os.environ.get('LSM_ARTIFACT_DIR', '/tmp/lsm-markers'))
artifacts.mkdir(parents=True, exist_ok=True)
linked = '# 링크 메모\n[예시 이름](https://example.com/path) 뒤\n\n- [ ] 할 일\n- 기호\n1. 번호\n**굵게** 끝\n'
ids = [str(uuid.uuid4()) for _ in range(5)]
texts = [linked, '캔버스 A\n- [ ] 첫째\n- [x] 둘째', '캔버스 B\n1. 번호', '캔버스 C', '캔버스 D']


LINKED_NOTE = (150, 190)
BULLET_BODY = (49, 182)
LINK_START = (50, 114)
LINK_TEXT = (60, 114)
OUTSIDE_PREVIEW = (400, 340)
PREVIEW_OPEN = (122, 193)
LINE_GAP_SWEEP_X = 300
LINE_GAP_SWEEP_Y = range(60, 300, 3)

def grab():
    return ImageGrab.grab(xdisplay=os.environ['DISPLAY'])


def changed(before, box):
    return ImageChops.difference(before.crop(box), grab().crop(box)).getbbox() is not None


with tempfile.TemporaryDirectory(prefix='lsm-markers-') as tmp:
    env = environment(tmp)
    path = write_notes(tmp, [
        dict(id=id, text=text, color='yellow', modified=1, canvas=dict(position=[64 + 300 * i, 64], next=[], done=False))
        for i, (id, text) in enumerate(zip(ids, texts))
    ])
    # 임시 URI 핸들러가 외부 브라우저 대신 주소를 기록한다.
    uri_log = uri_handler(tmp, env)
    config = Path(env['XDG_CONFIG_HOME']) / 'gtk-4.0'
    config.mkdir()
    (config / 'settings.ini').write_text('[Settings]\ngtk-cursor-blink=false\n')
    log = open(artifacts / 'markers-app.log', 'w')
    wm = sp.Popen([os.environ.get('LSM_OPENBOX', 'openbox'), '--sm-disable'], env=env, stdout=log, stderr=log)
    app = None
    try:
        time.sleep(.6)
        app = sp.Popen([binary], env=env, stdout=log, stderr=log)
        library = wait_window('^스티키 메모$')
        call('windowmove', library, '0', '0')
        call('windowfocus', library)
        move(library, *LINKED_NOTE)
        time.sleep(.2)
        click(library, *LINKED_NOTE)
        note = wait_window('^메모$')
        call('windowmove', note, '500', '60')
        call('windowsize', note, '500', '420')
        call('windowraise', note)
        call('windowfocus', note)
        key('ctrl+End')
        time.sleep(.4)
        grab().crop((500, 60, 1000, 480)).save(artifacts / 'markers-editor.png')
        # 숨긴 표식이 있는 줄의 모든 높이를 눌러도 앱이 죽지 않는다(줄 아래 간격 회귀).
        for py in LINE_GAP_SWEEP_Y:
            click(note, LINE_GAP_SWEEP_X, py)
            assert app.poll() is None, f'crash when clicking y={py}'
        key('ctrl+End')
        # 쓰는 흐름: 표식은 공백까지 치면 바로 렌더링되고 Enter로 이어지며 빈 항목에서 끝난다.
        key('ctrl+End')
        for keys in ['Return', 'minus', 'space']:
            key(keys)
        call('type', '--delay', '20', 'one')
        key('Return')
        call('type', '--delay', '20', 'two')
        key('Return', 'Return')
        key('minus', 'space', 'bracketleft', 'space', 'bracketright', 'space')
        call('type', '--delay', '20', 'task')
        key('Return', 'Return')
        time.sleep(.3)
        grab().crop((500, 60, 1000, 480)).save(artifacts / 'markers-typing.png')
        key('ctrl+a', 'ctrl+c')
        typed = sp.check_output([clip, '-selection', 'clipboard', '-o'], text=True, timeout=5)
        assert typed == linked + '\n- one\n- two\n- [ ] task\n', 'typing flow: ' + repr(typed)
        for _ in range(40):
            key('ctrl+a', 'ctrl+c')
            if sp.check_output([clip, '-selection', 'clipboard', '-o'], text=True, timeout=5) == linked:
                break
            key('ctrl+End', 'ctrl+z')
        else:
            raise AssertionError('undo never restored the note')
        key('ctrl+End')
        # 표식이 있는 항목의 본문 맨 앞을 누르고 입력해도 목록 문법 앞에 들어가지 않는다.
        # 왼쪽 여백 20 + 표식 칸 24 + 창 그림자 4 → 본문 시작 x≈48. y≈182가 '- 기호' 줄이다.
        click(note, *BULLET_BODY)
        time.sleep(.2)
        call('type', '--delay', '20', 'Q')
        time.sleep(.4)
        key('ctrl+a', 'ctrl+c')
        probe = sp.check_output([clip, '-selection', 'clipboard', '-o'], text=True, timeout=5)
        assert '- Q기호' in probe or '- [ ] Q할 일' in probe, 'typed before the hidden marker: ' + probe
        key('ctrl+z')
        time.sleep(.3)
        key('ctrl+End')
        time.sleep(.2)
        # 링크 머무르기: 카드가 나타나고, 지연 전에는 나타나지 않는다.
        box = (500, 150, 1000, 400)
        # 제목 줄(약 84) 다음 줄이 링크다. 링크 위치는 왼쪽 여백 바로 뒤다.
        before = grab()
        move(note, *LINK_START)
        time.sleep(.1)
        assert not changed(before, box), 'card must wait for hover intent'
        time.sleep(.6)
        assert changed(before, box), 'link hover shows the preview card'
        grab().save(artifacts / 'markers-link-card.png')
        # 카드가 떠 있어도 입력은 편집기로 가고, 카드는 물러난다.
        call('type', '--delay', '20', 'typed')
        time.sleep(.5)
        key('ctrl+a', 'ctrl+c')
        source = sp.check_output([clip, '-selection', 'clipboard', '-o'], text=True, timeout=5)
        assert source == linked + 'typed', 'typing while the card is shown reaches the note'
        key('ctrl+End')
        # 포인터가 멈춘 채라 다시 움직이기 전에는 뜨지 않는다.
        before = grab()
        time.sleep(.6)
        assert not changed(before, box), 'card stays away until the pointer moves'
        # Esc는 포인터를 옮기지 않고 닫는다.
        move(note, *LINK_TEXT)
        time.sleep(.7)
        shown = grab()
        key('Escape')
        time.sleep(.3)
        assert changed(shown, box), 'Esc dismisses the card'
        # 키보드만으로 링크 열기: 커서를 링크 안에 두고 Ctrl+Enter.
        key('ctrl+Home', 'Down', 'Right', 'Right')
        time.sleep(.3)
        key('ctrl+Return')
        for _ in range(60):
            if uri_log.exists():
                break
            time.sleep(.05)
        assert uri_log.read_text() == 'https://example.com/path', 'Ctrl+Enter opens the link under the caret'
        # 포인터를 카드로 옮겨도 닫히지 않고, 복사하면 원본 주소가 들어간다.
        # 외부 처리기를 띄운 뒤 창 포커스가 돌아오길 기다린다.
        time.sleep(1)
        call('windowfocus', note)
        time.sleep(.3)
        key('ctrl+End')
        # Esc로 닫은 링크는 포인터가 벗어났다 와야 다시 뜬다.
        move(note, *OUTSIDE_PREVIEW)
        time.sleep(.5)
        move(note, *LINK_START)
        time.sleep(.8)
        # 사람처럼 카드 위로 옮긴 뒤 누른다. 카드가 포인터를 받아 유지되는지도 함께 본다.
        move(note, *PREVIEW_OPEN)
        time.sleep(.5)
        click(note, *PREVIEW_OPEN)
        time.sleep(.2)
        copied = sp.check_output([clip, '-selection', 'clipboard', '-o'], text=True, timeout=5)
        assert copied == 'https://example.com/path', 'copy from the card: ' + copied
        move(note, *OUTSIDE_PREVIEW)
        time.sleep(.5)
        # 카드가 떠 있는 채로 창을 닫아도 앱이 멈추지 않는다.
        before = grab()
        move(note, *LINK_TEXT)
        time.sleep(.7)
        assert changed(before, box), 'card is up before closing'
        key('ctrl+w')
        time.sleep(.6)
        assert app.poll() is None, 'closing with a visible card must not crash'
        call('search', '--onlyvisible', '--name', '^스티키 메모$')

        # 캔버스
        call('windowfocus', library)
        click(library, *OPEN_CANVAS)
        canvas = wait_window('^메모 캔버스$')
        call('windowmove', canvas, '0', '0')
        call('windowsize', canvas, '1380', '760')
        call('windowfocus', canvas)
        time.sleep(.7)
        key('shift+1')
        time.sleep(.8)
        grab().save(artifacts / 'markers-canvas.png')
        # 전체 보기는 카드를 가로로 맞춘다. 화면 좌표 → 메모 번호.
        data = json.loads(path.read_text())
        assert [n['id'] for n in data] == ids
        def card(i):
            # 패널이 없을 때의 맞춤: 카드 폭 합을 창 안에 넣는다.
            width = 300 * 4 + 240
            zoom = min((1380 - 144) / width, 1)
            left = (1380 - width * zoom) / 2
            return int(left + (300 * i + 120) * zoom), 400
        def saved():
            return json.loads(path.read_text())
        click(canvas, *card(3))
        time.sleep(.3)
        key('Delete')
        time.sleep(.4)
        grab().save(artifacts / 'markers-confirm.png')
        # 기본 응답은 취소다.
        key('Return')
        time.sleep(.5)
        assert len(saved()) == 5, 'Enter on the default response cancels'
        key('Delete')
        time.sleep(.4)
        key('Escape')
        time.sleep(.5)
        assert len(saved()) == 5, 'Esc cancels'
        key('Delete')
        time.sleep(.4)
        key('Tab', 'Return')
        time.sleep(1)
        assert [n['id'] for n in saved()] == ids[:3] + ids[4:], 'Delete removes the selected note'
        # 되돌리기는 삭제한 메모를 되살리지 않는다.
        key('ctrl+z')
        time.sleep(.5)
        assert len(saved()) == 4
        # 완료 단축키는 저장되고 되돌릴 수 있다.
        key('shift+1')
        time.sleep(.8)
        click(canvas, *card(1))
        key('ctrl+Return')
        time.sleep(.6)
        assert saved()[1]['canvas']['done'] is True, 'Ctrl+Enter marks done'
        key('ctrl+z')
        time.sleep(.6)
        assert saved()[1]['canvas']['done'] is False, 'undo restores done'
        # 여러 개는 한 번의 확인으로 지운다.
        key('ctrl+a')
        time.sleep(.3)
        key('BackSpace')
        time.sleep(.4)
        grab().save(artifacts / 'markers-confirm-many.png')
        key('Tab', 'Return')
        time.sleep(1.5)
        assert saved() == [], 'Backspace removes every selected note'
        grab().save(artifacts / 'markers-canvas-empty.png')
        key('ctrl+w')
        time.sleep(.3)
        key('ctrl+w')
        assert app.wait(timeout=8) == 0
        text = (artifacts / 'markers-app.log').read_text()
        for bad in ['panicked at', 'Gtk-CRITICAL', 'Gtk-ERROR', 'is not a child', 'natural size must be']:
            assert bad not in text, bad
        print('PASS: gap clicks, link hover intent/typing/Esc/keyboard open/close, canvas Delete cancel-accept, undo boundary, Ctrl+Enter done, multi delete')
    finally:
        if app and app.poll() is None:
            app.terminate()
            app.wait(timeout=5)
        wm.terminate()
        wm.wait(timeout=5)
