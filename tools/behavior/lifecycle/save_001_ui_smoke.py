# 검증: SAVE-001; NOTE-001 생성·닫기·재열기, LSM_SMOKE_MARKDOWN=1이면 FMT-001 전환·원문 보존
# Run from the repository root under dbus-run-session and xvfb-run.
import subprocess as sp, tempfile, os, time, json
from pathlib import Path
markdown=os.environ.get('LSM_SMOKE_MARKDOWN') == '1'
text='# Heading\n## Subheading\n**bold** *italic* ~~strike~~\n- bullet\n1. number\n- [ ] todo\n- [x] done\n[link](https://example.com)' if markdown else 'Refactor smoke test'
from tools.behavior.lsm_driver import (call, wait_window as window, x, binary, environment, move, CLOSE, LIBRARY_NOTE, NEW_NOTE)
with tempfile.TemporaryDirectory(prefix='lsm-refactor-') as tmp:
 env=environment(tmp)
 log=open(tmp+'/test.log','w')
 wm=sp.Popen([os.environ.get('LSM_OPENBOX', 'openbox'),'--sm-disable'],env=env,stdout=log,stderr=log)
 app=None
 try:
  time.sleep(.6)
  if wm.poll() is not None: raise RuntimeError('Openbox failed; check its library and theme paths')
  app=sp.Popen([binary],env=env,stdout=log,stderr=log)
  print('wait library',flush=True);lib=window('^스티키 메모$');print('library',lib,flush=True);call('windowraise',lib);call('windowfocus',lib)
  move(lib,*NEW_NOTE);call('click','1')
  print('wait editor',flush=True);note=window('^메모$');print('editor',note,flush=True);call('windowraise',note);call('windowfocus',note)
  for index, line in enumerate(text.split('\n')):
   if index: call('key','Return')
   call('type','--clearmodifiers','--',line)
  if markdown:
   for _ in range(2):
    call('key','--clearmodifiers','ctrl+shift+m');time.sleep(.2)
  move(call('getwindowfocus'),*CLOSE);call('click','1')
  for _ in range(120):
   path=Path(tmp)/'linux-sticky-memo/notes.json'
   if path.exists() and json.loads(path.read_text())[0]['text']==text: break
   time.sleep(.05)
  else: raise AssertionError('close flush lost text: '+(path.read_text() if path.exists() else 'missing file'))
  call('windowraise',lib);call('windowfocus',lib);move(call('getwindowfocus'),*CLOSE);call('click','1');assert app.wait(timeout=8)==0
  app=sp.Popen([binary],env=env,stdout=log,stderr=log)
  print('wait library',flush=True);lib=window('^스티키 메모$');print('library',lib,flush=True);call('windowraise',lib);call('windowfocus',lib)
  move(lib,*LIBRARY_NOTE);call('click','1')
  print('wait editor',flush=True);note=window('^메모$');print('editor',note,flush=True);call('windowraise',note);call('windowfocus',note);move(call('getwindowfocus'),*CLOSE);call('click','1')
  call('windowraise',lib);call('windowfocus',lib);move(call('getwindowfocus'),*CLOSE);call('click','1');assert app.wait(timeout=8)==0
  assert json.loads(path.read_text())[0]['text']==text
  log.flush()
  assert 'Failed to set text' not in Path(tmp+'/test.log').read_text()
  print('PASS: create, edit, immediate close flush, normal exit, restart, reopen persisted note')
 except Exception:
  log.flush();print(Path(tmp+'/test.log').read_text());print(sp.run([x,'search','--name','.*'],capture_output=True,text=True).stdout);raise
 finally:
  if app and app.poll() is None: app.terminate();app.wait(timeout=5)
  wm.terminate();wm.wait(timeout=5)
