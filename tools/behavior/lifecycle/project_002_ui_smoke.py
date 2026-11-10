"""PROJECT-002: real GTK pointer/keyboard, search, restart and note preservation."""
import json
import os
from pathlib import Path
import subprocess as sp
import tempfile
import time
import uuid
from PIL import ImageGrab, ImageChops
from tools.behavior.memoivy_driver import environment, write_notes, wait_window, call, click, key, binary

artifacts = Path(os.environ.get('MEMOIVY_ARTIFACT_DIR', '/tmp/memoivy-library-e2e'))
artifacts.mkdir(parents=True, exist_ok=True)
with tempfile.TemporaryDirectory(prefix='memoivy-library-') as tmp:
    env = environment(tmp)
    parent, nested, leaf, loose = [str(uuid.uuid4()) for _ in range(4)]
    notes = [dict(id=id, text=text, color='yellow', modified=modified,
                  canvas=dict(parent=pid, position=[64, 64], next=[], done=False))
             for id, text, pid, modified in [
                 (parent, 'Project Alpha', None, 1),
                 (nested, 'Nested project', parent, 2),
                 (leaf, 'Needle child', nested, 3),
                 (loose, 'Loose memo', None, 4)]]
    path = write_notes(tmp, notes)
    prefs = path.with_name('library.json')
    log = open(artifacts / 'app.log', 'w')
    wm = sp.Popen([os.environ.get('MEMOIVY_OPENBOX', 'openbox'), '--sm-disable'], env=env, stdout=log, stderr=log)
    app = None
    def start():
        global app
        app = sp.Popen([binary], env=env, stdout=log, stderr=log)
        window = wait_window('^MemoIvy$')
        call('windowmove', window, '0', '0')
        call('windowfocus', window)
        time.sleep(.5)
        return window
    def stop(window):
        click(window, 34, 26)
        app.wait(timeout=6)
    def image(window, name, rows=None):
        call('mousemove', '900', '700')
        time.sleep(.25)
        geom = dict(line.split('=', 1) for line in call('getwindowgeometry', '--shell', window).splitlines())
        x, y, width, height = [int(geom[k]) for k in ('X', 'Y', 'WIDTH', 'HEIGHT')]
        shot = ImageGrab.grab().crop((x, y, x+width, y+height))
        shot.save(artifacts / (name + '.png'))
        if rows is not None:
            # Count the fixture's yellow card stripes, independently of saved state.
            bands = 0
            previous = False
            for py in range(145, shot.height):
                stripe = any(r > 150 and g > 110 and r > b + 50 and g > b + 30
                             for r, g, b in [shot.getpixel((px, py))[:3] for px in range(20, 100)])
                if stripe and not previous:
                    bands += 1
                previous = stripe
            assert bands == rows, f'{name}: expected {rows} rendered cards, got {bands}'
        return shot
    def saved(expected):
        assert prefs.exists(), 'expansion must persist immediately'
        assert set(json.loads(prefs.read_text())) == set(expected), 'wrong persisted expansion'
    try:
        time.sleep(.5)
        library = start()
        assert not prefs.exists(), 'first render must not write preferences'
        initial = image(library, '01-default-collapsed', 2)
        click(library, 48, 195)
        saved([parent])
        expanded = image(library, '02-parent-expanded', 3)
        assert ImageChops.difference(initial.crop((20, 245, 415, 380)), expanded.crop((20, 245, 415, 380))).getbbox(), 'click must reveal nested project'
        # Focus survives rebuild; Space toggles exactly the same disclosure.
        key('space')
        saved([])
        key('space')
        saved([parent])
        key('Return')
        saved([])
        key('Return')
        saved([parent])
        click(library, 66, 277)
        saved([parent, nested])
        image(library, '03-nested-expanded', 4)
        # Collapse ancestor without erasing the child's remembered choice.
        click(library, 48, 195)
        saved([nested])
        click(library, 100, 75)
        call('type', '--clearmodifiers', 'Needle')
        time.sleep(.6)
        image(library, '04-search-hidden-child', 1)
        saved([nested])
        key('ctrl+a'); key('BackSpace'); time.sleep(.5)
        image(library, '05-search-cleared', 2)
        saved([nested])
        # Recent view must not rewrite tree state.
        click(library, 386, 127)
        image(library, '06-recent', 4)
        saved([nested])
        click(library, 330, 127)
        stop(library)
        library = start()
        saved([nested])
        image(library, '07-restarted-collapsed', 2)
        click(library, 48, 195)
        saved([parent, nested])
        image(library, '08-restored-nested', 4)
        stop(library)
        library = start()
        image(library, '09-restarted-expanded', 4)
        saved([parent, nested])
        call('windowsize', library, '360', '600')
        image(library, '10-narrow', 4)
        stop(library)
        # Corrupted auxiliary state must not prevent loading notes.
        prefs.write_text('{broken')
        library = start()
        image(library, '11-corrupt-state-recovered', 2)
        click(library, 48, 195)
        saved([parent])
        stop(library)
        # Force an atomic-save failure, then restore the destination and retry.
        prefs.unlink()
        prefs.mkdir()
        library = start()
        click(library, 48, 195)
        dialog = wait_window('^목록 상태 저장 오류$')
        image(dialog, '12-save-failure')
        call('windowfocus', dialog)
        key('Escape')
        image(library, '13-failed-save-keeps-session', 3)
        prefs.rmdir()
        click(library, 48, 195)
        saved([])
        click(library, 48, 195)
        saved([parent])
        image(library, '14-save-recovered', 3)
        stop(library)
        stored = json.loads(path.read_text())
        assert [(n['id'], n['text'], n['canvas']['parent']) for n in stored] == [(n['id'], n['text'], n['canvas']['parent']) for n in notes], 'view interactions must preserve notes and nesting'
        assert prefs.stat().st_mode & 0o077 == 0, 'preferences must be owner-only'
        print('PASS: disclosure click/Space, nested memory, search/recent isolation, restart, narrow window, corrupt recovery, save failure/retry, note preservation')
        print('Screenshots:', artifacts)
    finally:
        if app and app.poll() is None:
            app.terminate(); app.wait(timeout=5)
        wm.terminate(); wm.wait(timeout=5)
        log.close()
