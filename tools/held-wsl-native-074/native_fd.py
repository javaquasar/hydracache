"""Single synthetic native FD mapping control in the new local WSL only."""
import json
from pathlib import Path
import secrets
import subprocess
import prepare as p
import host

OUTPUT = p.ROOT / 'target/held-native-fd-20261022'
SCRIPT = '/mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/held-wsl-native-074/native_fd.sh'


def argv(nonce):
    p.need(len(nonce) == 32 and all(c in '0123456789abcdef' for c in nonce), 'Unsafe nonce')
    return ['wsl.exe', '-d', p.DISTRO, '-u', 'root', '--cd', '/', '--exec', '/bin/bash', SCRIPT, nonce]


def run():
    p.need(not OUTPUT.exists(), 'Never resume native FD control')
    p.need(json.loads((p.OUTPUT / 'boot-result.json').read_text())['success'] is True, 'Environment not verified')
    before = host.snapshot()
    host.preserved(json.loads((p.OUTPUT / 'import-before.json').read_text()), before)
    p.need(subprocess.check_output(['git', 'status', '--porcelain'], cwd=p.ROOT) == b'', 'Commit exact source first')
    OUTPUT.mkdir()
    nonce = secrets.token_hex(16)
    command = argv(nonce)
    p.publish(OUTPUT / 'intent.json', dict(argv=command, nonce=nonce, original_delivery=False,
        writers_cessation=False, admission_allowed=False, source_commit=subprocess.check_output(
            ['git', 'rev-parse', 'HEAD'], cwd=p.ROOT, text=True).strip(),
        source_sha256={f.name: p.sha(f) for f in Path(__file__).parent.iterdir() if f.is_file()}))
    # The shell records every bus command. There is one StartTransientUnit and
    # no automatic retry, fallback, arbitrary unit or user-provided executable.
    result = dict(failure=None, original_delivery=False, writers_cessation=False, admission_allowed=False)
    try:
        child = subprocess.run(command, capture_output=True, timeout=30, cwd=p.ROOT)
        stdout, stderr = child.stdout, child.stderr
        result['exit'] = child.returncode
    except subprocess.TimeoutExpired as error:
        stdout, stderr = error.stdout or b'', error.stderr or b''
        result.update(exit=None, failure='TimeoutExpired; inspect exact unit, do not resend')
    for name, raw in (('stdout.log', stdout), ('stderr.log', stderr)):
        with (OUTPUT / name).open('xb') as stream: stream.write(raw)
    after = host.snapshot()
    p.publish(OUTPUT / 'host-after.json', after)
    host.preserved(json.loads((p.OUTPUT / 'import-before.json').read_text()), after)
    p.publish(OUTPUT / 'result.json', result)
    print(json.dumps(result, sort_keys=True))
    p.need(result['exit'] == 0, 'Native FD control failed; first outcome retained, no resend')


if __name__ == '__main__': run()
