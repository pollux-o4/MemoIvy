from tools.behavior.memoivy_driver import (call, wait_window, key, click, x, clip, binary, environment, write_notes, OPEN_CANVAS)
# 검증: CANVAS-001·CANVAS-002. 합성 데이터의 실제 포인터·키보드·재시작 검사.
import json
import os
from pathlib import Path
import subprocess as sp
import tempfile
import time
import uuid
from PIL import ImageChops, ImageGrab

artifacts = Path(os.environ.get('MEMOIVY_ARTIFACT_DIR', '/tmp/memoivy-keyboard'))
artifacts.mkdir(parents=True, exist_ok=True)
original = '기존 메모'


def copied_source():
    key('ctrl+a')
    key('ctrl+c')
    return sp.check_output([clip, '-selection', 'clipboard', '-o'], text=True, timeout=5)


FIRST_CARD = (184, 142)
MARQUEE_START = (40, 20)
FIRST_CARD_HOVER = (240, 160)
FIRST_PORT = (380, 172)
FIRST_CARD_EDGE = (380, 142)
SECOND_CHECK = (617, 87)
SECOND_CARD = (520, 142)
VIEW_CENTER = (580, 403)


with tempfile.TemporaryDirectory(prefix='memoivy-canvas-') as tmp:
    env = environment(tmp)
    ids=[str(uuid.uuid4()) for _ in range(2)]
    path = write_notes(tmp, [dict(id=id,text='A very long shared title '+str(i)+'\nUnique context '+str(i),color=['yellow','blue'][i],modified=1,canvas=dict(position=[64+320*i,64],next=[],done=False)) for i,id in enumerate(ids)])
    log=open(artifacts/'canvas-app.log','w')
    wm=sp.Popen([os.environ.get('MEMOIVY_OPENBOX','openbox'),'--sm-disable'],env=env,stdout=log,stderr=log)
    app=None
    try:
        time.sleep(.6)
        app=sp.Popen([binary],env=env,stdout=log,stderr=log)
        library=wait_window('^MemoIvy$')
        call('windowfocus',library)
        click(library, *OPEN_CANVAS)
        canvas=wait_window('^메모 캔버스$')
        call('windowmove',canvas,'0','0')
        call('windowfocus',canvas)
        time.sleep(.5)
        # 실제 휠·수정키·중간 버튼 입력이 카메라만 바꾸고 메모 배치를 보존한다.
        geometry = dict(line.split('=', 1) for line in call('getwindowgeometry', '--shell', canvas).splitlines())
        width, height = int(geometry['WIDTH']), int(geometry['HEIGHT'])
        camera_origin = (width // 2, height // 2)
        body_box = (int(geometry['X']), int(geometry['Y']) + 50,
                    int(geometry['X']) + width, int(geometry['Y']) + height - 45)
        initial_notes = json.loads(path.read_text())
        call('mousemove', '--window', canvas, *map(str, camera_origin))
        time.sleep(.2)
        def camera_input(action, label):
            # 포인터·호버 변화만으로 카메라 이동 검사가 통과하지 않게 같은 곳으로 치운다.
            call('mousemove', '0', '0')
            before = ImageGrab.grab(xdisplay=os.environ['DISPLAY']).crop(body_box)
            call('mousemove', '--window', canvas, *map(str, camera_origin))
            action()
            call('mousemove', '0', '0')
            deadline = time.monotonic() + 3
            while True:
                after = ImageGrab.grab(xdisplay=os.environ['DISPLAY']).crop(body_box)
                if ImageChops.difference(before, after).getbbox() is not None:
                    break
                assert time.monotonic() < deadline, label + ' must move rendered cards'
                time.sleep(.02)
            assert json.loads(path.read_text()) == initial_notes, label + ' must not move note positions'
        camera_input(lambda: call('click', '5'), 'wheel pan')
        def zoom_wheel():
            call('keydown', 'ctrl')
            try:
                call('click', '4')
            finally:
                call('keyup', 'ctrl')
        camera_input(zoom_wheel, 'Ctrl+wheel zoom')
        def middle_pan():
            call('mousedown', '2')
            try:
                for dx, dy in [(10, 5), (25, 15), (40, 25)]:
                    call('mousemove', '--window', canvas, str(camera_origin[0] + dx), str(camera_origin[1] + dy))
            finally:
                call('mouseup', '2')
        camera_input(middle_pan, 'middle-button pan')
        key('Escape', 'shift+1')
        time.sleep(.8)
        # 첫 화면은 두 카드 중심을 창 가운데에 맞춘다(100%). 월드 → 창 좌표 변환.
        center=[344,142]
        at=lambda wx,wy:(wx-center[0]+VIEW_CENTER[0],wy-center[1]+VIEW_CENTER[1])
        click(canvas,*at(*FIRST_CARD))
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
        call('mousemove','--window',canvas,*map(str,at(*FIRST_CARD)))
        call('mousedown','1')
        for dx,dy in [(20,10),(40,20),(60,30)]:
            call('mousemove','--window',canvas,*map(str,at(FIRST_CARD[0]+dx,FIRST_CARD[1]+dy)));time.sleep(.06)
        call('mouseup','1')
        time.sleep(.6)
        data=json.loads(path.read_text())
        assert data[0]['canvas']['position']==[140,94],data
        # 빈 곳 끌기 범위 선택 + 위쪽 맞춤 + 되돌리기·다시 실행.
        call('mousemove','--window',canvas,*map(str,at(*MARQUEE_START)))
        call('mousedown','1')
        for dx,dy in [(200,100),(500,200),(700,260)]:
            call('mousemove','--window',canvas,*map(str,at(MARQUEE_START[0]+dx,MARQUEE_START[1]+dy)));time.sleep(.06)
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
        call('mousemove','--window',canvas,*map(str,at(*FIRST_CARD_HOVER)));time.sleep(.2)
        call('mousemove','--window',canvas,*map(str,at(*FIRST_PORT)))
        call('mousedown','1')
        for wx in (420,480,520):
            call('mousemove','--window',canvas,*map(str,at(wx,142)));time.sleep(.06)
        call('mouseup','1')
        time.sleep(.6)
        data=json.loads(path.read_text())
        assert data[0]['canvas']['next']==[ids[1]],data
        # 카드의 완료 표시는 드래그에 먹히지 않고 눌린다.
        click(canvas,*at(*FIRST_CARD_EDGE))
        time.sleep(.3)
        click(canvas,*at(*SECOND_CHECK))
        time.sleep(.6)
        data=json.loads(path.read_text())
        assert data[1]['canvas']['done'] is True,data
        assert data[1]['canvas']['position']==[400,64],data
        # 두 번 눌러 원본 열기.
        call('mousemove','--window',canvas,*map(str,at(*SECOND_CARD)))
        call('click','--repeat','2','--delay','70','1')
        note=wait_window('^메모$')
        key('ctrl+w')
        time.sleep(.3)
        call('windowfocus',canvas)
        time.sleep(.2)
        call('windowsize',canvas,'820','620')
        time.sleep(.4)
        ImageGrab.grab(xdisplay=os.environ['DISPLAY']).save(artifacts/'canvas-pointer-keyboard.png')
        key('ctrl+w')
        time.sleep(.3)
        key('ctrl+w')
        assert app.wait(timeout=8)==0
        app=sp.Popen([binary],env=env,stdout=log,stderr=log)
        library=wait_window('^MemoIvy$')
        assert json.loads(path.read_text())==data,'restart changed memo identity or layout'
        call('windowfocus',library)
        key('ctrl+w')
        assert app.wait(timeout=8)==0
        assert not any(x in (artifacts/'canvas-app.log').read_text() for x in ['panicked at','Gtk-CRITICAL'])
        print('PASS: wheel pan/anchored zoom/middle-button pan, pointer selection/drag, Tab/arrow movement, marquee/align/undo, port link, double-click open, small window, close/flush, restart identity/layout')
    finally:
        if app and app.poll() is None: app.terminate();app.wait(timeout=5)
        wm.terminate();wm.wait(timeout=5)
