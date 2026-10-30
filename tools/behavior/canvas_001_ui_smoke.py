# 검증: CANVAS-001·CANVAS-002. 합성 데이터의 실제 포인터·키보드·재시작 검사.
import json
import os
from pathlib import Path
import subprocess as sp
import tempfile
import time
import uuid

x = os.environ.get('LSM_XDOTOOL', 'xdotool')
clip = os.environ.get('LSM_XCLIP', 'xclip')
binary = os.environ.get('LSM_BINARY', 'target/debug/linux-sticky-memo')
artifacts = Path(os.environ.get('LSM_ARTIFACT_DIR', '/tmp/lsm-keyboard'))
artifacts.mkdir(parents=True, exist_ok=True)
original = '기존 메모'


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


def copied_source():
    key('ctrl+a')
    key('ctrl+c')
    return sp.check_output([clip, '-selection', 'clipboard', '-o'], text=True, timeout=5)


with tempfile.TemporaryDirectory(prefix='lsm-canvas-') as tmp:
    env = dict(os.environ,GDK_BACKEND='x11',GDK_DEBUG='no-portals',GSK_RENDERER='cairo',GTK_USE_PORTAL='0',GIO_USE_VFS='local',GTK_A11Y='none',XDG_DATA_HOME=tmp,XDG_CONFIG_HOME=tmp+'/config')
    path=Path(tmp)/'linux-sticky-memo/notes.json'
    path.parent.mkdir()
    ids=[str(uuid.uuid4()) for _ in range(2)]
    path.write_text(json.dumps([dict(id=id,text='A very long shared title '+str(i)+'\nUnique context '+str(i),color=['yellow','blue'][i],modified=1,canvas=dict(position=[64+320*i,64],next=[],done=False)) for i,id in enumerate(ids)]))
    log=open(artifacts/'canvas-app.log','w')
    wm=sp.Popen([os.environ.get('LSM_OPENBOX','openbox'),'--sm-disable'],env=env,stdout=log,stderr=log)
    app=None
    try:
        time.sleep(.6)
        app=sp.Popen([binary],env=env,stdout=log,stderr=log)
        library=wait_window('^스티키 메모$')
        call('windowfocus',library)
        click(library,300,24)
        canvas=wait_window('^메모 캔버스$')
        call('windowmove',canvas,'0','0')
        call('windowfocus',canvas)
        time.sleep(.5)
        # 첫 화면은 두 카드 중심을 창 가운데에 맞춘다(100%). 월드 → 창 좌표 변환.
        center=[344,142]
        at=lambda wx,wy:(wx-center[0]+580,wy-center[1]+403)
        click(canvas,*at(184,142))
        key('Right')
        time.sleep(.6)
        data=json.loads(path.read_text())
        assert data[0]['canvas']['position']==[80,64],data
        # Tab은 다음 카드로 이동하고 방향키는 그 카드를 옮긴다.
        key('Tab','Right')
        time.sleep(.6)
        data=json.loads(path.read_text())
        assert data[1]['canvas']['position']==[400,64],data
        # Tab은 패널에 가린 카드를 보이는 곳으로 옮긴다. 선택을 풀고 전체 보기로 좌표를 다시 잡는다.
        key('Escape','shift+1')
        time.sleep(.8)
        center[0]=360
        # 카드 전체 끌기.
        call('mousemove','--window',canvas,*map(str,at(184,142)))
        call('mousedown','1')
        for dx,dy in [(20,10),(40,20),(60,30)]:
            call('mousemove','--window',canvas,*map(str,at(184+dx,142+dy)));time.sleep(.06)
        call('mouseup','1')
        time.sleep(.6)
        data=json.loads(path.read_text())
        assert data[0]['canvas']['position']==[140,94],data
        # 빈 곳 끌기 범위 선택 + 위쪽 맞춤 + 되돌리기·다시 실행.
        call('mousemove','--window',canvas,*map(str,at(40,20)))
        call('mousedown','1')
        for dx,dy in [(200,100),(500,200),(700,260)]:
            call('mousemove','--window',canvas,*map(str,at(40+dx,20+dy)));time.sleep(.06)
        call('mouseup','1')
        time.sleep(.3)
        key('alt+w')
        time.sleep(.6)
        data=json.loads(path.read_text())
        assert [n['canvas']['position'][1] for n in data]==[64,64],data
        key('ctrl+z')
        time.sleep(.6)
        data=json.loads(path.read_text())
        assert data[0]['canvas']['position']==[140,94],data
        key('Escape')
        time.sleep(.2)
        # 연결 점 끌기로 연결, 맞춤선 없이 놓기.
        call('mousemove','--window',canvas,*map(str,at(240,160)));time.sleep(.2)
        call('mousemove','--window',canvas,*map(str,at(380,172)))
        call('mousedown','1')
        for wx in (420,480,520):
            call('mousemove','--window',canvas,*map(str,at(wx,142)));time.sleep(.06)
        call('mouseup','1')
        time.sleep(.6)
        data=json.loads(path.read_text())
        assert data[0]['canvas']['next']==[ids[1]],data
        # 카드의 완료 표시는 드래그에 먹히지 않고 눌린다.
        click(canvas,*at(380,142))
        time.sleep(.3)
        click(canvas,*at(617,87))
        time.sleep(.6)
        data=json.loads(path.read_text())
        assert data[1]['canvas']['done'] is True,data
        assert data[1]['canvas']['position']==[400,64],data
        # 두 번 눌러 원본 열기.
        call('mousemove','--window',canvas,*map(str,at(520,142)))
        call('click','--repeat','2','--delay','70','1')
        note=wait_window('^메모$')
        key('ctrl+w')
        time.sleep(.3)
        call('windowfocus',canvas)
        time.sleep(.2)
        call('windowsize',canvas,'820','620')
        time.sleep(.4)
        from PIL import ImageGrab
        ImageGrab.grab(xdisplay=os.environ['DISPLAY']).save(artifacts/'canvas-pointer-keyboard.png')
        key('ctrl+w')
        time.sleep(.3)
        key('ctrl+w')
        assert app.wait(timeout=8)==0
        app=sp.Popen([binary],env=env,stdout=log,stderr=log)
        library=wait_window('^스티키 메모$')
        assert json.loads(path.read_text())==data,'restart changed memo identity or layout'
        call('windowfocus',library)
        key('ctrl+w')
        assert app.wait(timeout=8)==0
        assert not any(x in (artifacts/'canvas-app.log').read_text() for x in ['panicked at','Gtk-CRITICAL'])
        print('PASS: pointer selection/drag, Tab/arrow movement, marquee/align/undo, port link, double-click open, small window, close/flush, restart identity/layout')
    finally:
        if app and app.poll() is None: app.terminate();app.wait(timeout=5)
        wm.terminate();wm.wait(timeout=5)
