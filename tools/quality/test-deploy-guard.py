"""배포 차단·순서를 검증한다. 패키징과 업로드는 가짜 명령만 실행한다."""
import os
from pathlib import Path
import subprocess
import tempfile


script = Path(__file__).resolve().parents[1] / "deploy-snap.sh"
with tempfile.TemporaryDirectory() as folder:
    root = Path(folder)
    subprocess.run(["git", "init", "-q", folder], check=True)
    (root / "Cargo.toml").write_text('[package]\nversion = "9.9.9"\n')
    subprocess.run(["git", "-C", folder, "add", "."], check=True)
    subprocess.run([
        "git", "-C", folder, "-c", "user.name=Test",
        "-c", "user.email=test@example.com", "commit", "-qm", "fixture",
    ], check=True)
    bindir = root / ".git/bin"
    bindir.mkdir()
    log = root / ".git/calls"
    commands = {
        "mise": '''echo "$*" >> "$CALLS"
if [ "$*" = "$FAIL_STEP" ]; then exit 1; fi
if [ "$*" = "run package:snap" ]; then touch memoivy_9.9.9_amd64.snap; fi
''',
        "snapcraft": 'echo upload >> "$CALLS"\n',
    }
    for name, body in commands.items():
        command = bindir / name
        command.write_text("#!/bin/sh\n" + body)
        command.chmod(0o755)
    env = dict(os.environ, PATH=f"{bindir}:{os.environ['PATH']}", CALLS=str(log))

    def run(fail_step=""):
        return subprocess.run(
            ["bash", str(script)], cwd=root,
            env=dict(env, FAIL_STEP=fail_step), capture_output=True,
        ).returncode

    (root / "untracked").touch()
    assert run() != 0 and not log.exists(), "미추적 파일 차단"
    (root / "untracked").unlink()
    (root / "Cargo.toml").write_text("dirty")
    assert run() != 0 and not log.exists(), "미커밋 변경 차단"
    subprocess.run(["git", "-C", folder, "restore", "Cargo.toml"], check=True)
    steps = ["run check:secrets", "run test:snap-gtk", "run package:snap"]
    for index, step in enumerate(steps):
        assert run(step) != 0, step
        assert log.read_text().splitlines() == steps[:index + 1], step
        log.unlink()
    assert run() == 0
    assert log.read_text().splitlines() == steps + ["upload"]
print("배포 차단·실행 순서 통과 (실제 업로드 없음)")
