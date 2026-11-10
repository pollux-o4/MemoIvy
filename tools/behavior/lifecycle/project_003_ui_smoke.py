"""PROJECT-003: New morph, keyboard/pointer creation, empty project and restart."""
import json
import os
from pathlib import Path
import subprocess as sp
import tempfile
import time
import uuid
from PIL import ImageGrab, ImageChops
from tools.behavior.memoivy_driver import environment, write_notes, wait_window, call, click, key, binary, NEW, OPEN_CANVAS

artifacts = Path(os.environ.get('MEMOIVY_ARTIFACT_DIR', '/tmp/memoivy-creation-e2e'))
artifacts.mkdir(parents=True, exist_ok=True)
with tempfile.TemporaryDirectory(prefix='memoivy-creation-') as tmp:
    env = environment(tmp)
    old = dict(id=str(uuid.uuid4()), text='Existing note', color='yellow', modified=1,
               canvas=dict(position=[64, 64], next=[], done=False))
    path = write_notes(tmp, [old])
    log = open(artifacts / 'app.log', 'w')
    wm = sp.Popen([os.environ.get('MEMOIVY_OPENBOX', 'openbox'), '--sm-disable'], env=env, stdout=log, stderr=log)
    compositor = sp.Popen([os.environ['MEMOIVY_XCOMPMGR'], '-n'], env=env, stdout=log, stderr=log) if os.environ.get('MEMOIVY_XCOMPMGR') else None
    app = None
    def shot(window, name):
        geom = dict(line.split('=', 1) for line in call('getwindowgeometry', '--shell', window).splitlines())
        x, y, w, h = [int(geom[k]) for k in ('X', 'Y', 'WIDTH', 'HEIGHT')]
        image = ImageGrab.grab().crop((x, y, x+w, y+h))
        image.save(artifacts / (name+'.png'))
        return image
    def stored(count):
        deadline = time.monotonic()+5
        while time.monotonic() < deadline:
            notes = json.loads(path.read_text())
            if len(notes) == count:
                return notes
            time.sleep(.05)
        raise AssertionError(f'expected {count} saved notes')
    def canvas(library):
        click(library, *OPEN_CANVAS)
        window = wait_window('^메모 캔버스$')
        call('windowmove', window, '0', '0')
        call('windowfocus', window)
        time.sleep(.6)
        return window
    try:
        time.sleep(.4)
        app = sp.Popen([binary], env=env, stdout=log, stderr=log)
        library = wait_window('^MemoIvy$')
        call('windowmove', library, '0', '0')
        time.sleep(.4)
        library_initial = shot(library, '01-canvas-svg')
        click(library, *NEW)
        time.sleep(.06)
        shot(library, '01-library-morph-early')
        time.sleep(.3)
        expanded = shot(library, '01-library-new-menu')
        assert ImageChops.difference(library_initial, expanded).getbbox(), 'library New must open the chooser'
        key('Escape')
        time.sleep(.35)
        assert len(json.loads(path.read_text())) == 1, 'opening library New must not create a memo'
        board = canvas(library)
        initial = shot(board, '02-new-button')
        # Capture real intermediate frames rather than drawing a motion mockup.
        call('mousemove', '--window', board, '1120', '26')
        call('click', '--delay', '0', '1')
        start = time.monotonic()
        frames = []
        for i, delay in enumerate([.035, .10, .17, .24, .34]):
            time.sleep(max(0, start+delay-time.monotonic()))
            frame = shot(board, f'03-motion-{i}')
            frames.append(frame.crop((930, 0, 1160, 160)))
        assert ImageChops.difference(frames[0], frames[-1]).getbbox(), 'morph must have intermediate geometry'
        call('key', '--clearmodifiers', 'Escape')
        start = time.monotonic()
        closing = []
        for i, delay in enumerate([.035, .10, .17, .24, .34]):
            time.sleep(max(0, start+delay-time.monotonic()))
            closing.append(shot(board, f'03-close-motion-{i}').crop((930, 0, 1160, 160)))
        animation = [initial.crop((930, 0, 1160, 160))]+frames+closing
        animation[0].save(artifacts/'new-morph.gif', save_all=True, append_images=animation[1:], duration=[400]+[70]*4+[400]+[70]*4+[400], loop=0)
        shot(board, '04-escape-closed')
        click(board, 1120, 26)
        # Focus starts on Memo; Enter creates it without moving the board selection.
        key('Return')
        editor = wait_window('^메모$')
        call('windowfocus', editor)
        key('ctrl+a')
        call('type', '--clearmodifiers', 'Menu memo')
        click(editor, 34, 26)
        stored(2)
        call('windowfocus', board)
        click(board, 1120, 26)
        time.sleep(.3)
        shot(board, '05-menu-open')
        click(board, 1020, 104)
        project_editor = wait_window('^프로젝트$')
        call('windowfocus', project_editor)
        key('ctrl+a')
        call('type', '--clearmodifiers', 'Empty project')
        click(project_editor, 34, 26)
        notes = stored(3)
        project = next(n for n in notes if n['text']=='Empty project')
        assert project['canvas']['project'] is True
        assert not any(n.get('canvas', {}).get('parent')==project['id'] for n in notes), 'project must start empty'
        # Outside click closes the chooser without creating another object.
        call('windowfocus', board)
        click(board, 1120, 26)
        click(board, 800, 200)
        shot(board, '06-outside-closed')
        assert len(stored(3)) == 3
        # New note shortcut retains its immediate-create behavior.
        call('windowfocus', board)
        key('ctrl+n')
        editor = wait_window('^메모$')
        call('windowfocus', editor)
        call('type', '--clearmodifiers', 'Shortcut memo')
        click(editor, 34, 26)
        stored(4)
        shot(board, '07-empty-project-card')
        click(board, 34, 26)
        call('windowfocus', library)
        time.sleep(.5)
        shot(library, '08-project-in-library')
        click(library, 34, 26)
        app.wait(timeout=6)
        app = sp.Popen([binary], env=env, stdout=log, stderr=log)
        library = wait_window('^MemoIvy$')
        call('windowmove', library, '0', '0'); time.sleep(.5)
        shot(library, '09-restarted-empty-project')
        board = canvas(library)
        shot(board, '10-restarted-canvas')
        notes = stored(4)
        assert next(n for n in notes if n['id']==project['id'])['canvas']['project'] is True
        assert next(n for n in notes if n['id']==old['id'])['text']==old['text']
        click(board, 34, 26)
        click(library, 34, 26)
        app.wait(timeout=6)
        assert 'Gtk-WARNING' not in (artifacts/'app.log').read_text(), 'GTK layout warning'
        print('PASS: real morph frames, Escape/outside close, Enter/pointer creation, Ctrl+N, atomic empty project, restart and source preservation')
        print('Screenshots:', artifacts)
    finally:
        if app and app.poll() is None:
            app.terminate(); app.wait(timeout=5)
        wm.terminate(); wm.wait(timeout=5)
        if compositor:
            compositor.terminate(); compositor.wait(timeout=5)
        log.close()
