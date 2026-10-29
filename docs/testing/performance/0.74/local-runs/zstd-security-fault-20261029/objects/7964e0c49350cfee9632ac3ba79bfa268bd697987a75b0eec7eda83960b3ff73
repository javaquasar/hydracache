"""Freeze/stage exactly one current release-image preparation profile on ax42."""
import ast
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import shutil
import subprocess
import sys
import tarfile

ROOT = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent
OUT = ROOT / 'target/server-preparation-profile-20261028'
PARENT = ROOT / 'target/dbus-original-delivery-20261028/unit-api-owned'
IMAGE = ROOT / 'target/original-manager-live-20261028/baseline-once/libtest'
IMAGE_SHA = 'd25675c71977c4dc48d4cbdf2973081f5d37189c4339e7de5465f571e2ad94c7'
SSH = ['ssh', '-T', '-o', 'BatchMode=yes', '-o', 'StrictHostKeyChecking=yes',
       '-o', 'ConnectTimeout=15', '-o', 'ServerAliveInterval=5', '-o',
       'ServerAliveCountMax=2', 'hydracache-ax42']
SCP = ['scp', '-o', 'BatchMode=yes', '-o', 'StrictHostKeyChecking=yes',
       '-o', 'ConnectTimeout=15']
NAMES = ('timing-controls-074', 'build-receipt-v1.json', 'build-log.jsonl',
         'Cargo.lock.root', 'Cargo.lock.observer', 'embedded.json', 'direct.json',
         'resp2.json', 'resp3.json')


def need(ok, reason):
    if not ok:
        raise ValueError(reason)


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def put(path, value):
    with path.open('x', encoding='utf8', newline='\n') as stream:
        json.dump(value, stream, indent=2, sort_keys=True)
        stream.write('\n')
        stream.flush()
        os.fsync(stream.fileno())


def call(label, command, timeout=60, data=None):
    put(OUT / (label + '-intent.json'), dict(argv=command, bound_seconds=timeout, retry=False))
    with (OUT / (label + '.stdout.log')).open('xb') as stdout, (OUT / (label + '.stderr.log')).open('xb') as stderr:
        try:
            code = subprocess.run(command, input=data, stdout=stdout, stderr=stderr,
                                  cwd=ROOT, timeout=timeout, check=False).returncode
        except subprocess.TimeoutExpired:
            code = 124
    put(OUT / (label + '-result.json'), dict(exit=code, retry=False))
    need(code == 0, label + ': first failure retained; inspect, do not resubmit')


def freeze():
    review = json.loads((PARENT / 'freeze/result.json').read_bytes())
    actual = {p.relative_to(PARENT / 'input').as_posix(): sha(p)
              for p in (PARENT / 'input').rglob('*') if p.is_file()}
    need(actual == review['files'], 'whole previously reviewed build source changed')
    need(all(sha(ROOT / name) == value for name, value in review['root_inputs'].items()),
         'protected root input changed')
    proof = json.loads((ROOT / 'target/original-manager-live-20261028/build/native-result.json').read_bytes())
    need(proof['passed'] and proof['source_unchanged']
         and proof['artifacts']['libtest']['sha256'] == sha(IMAGE) == IMAGE_SHA,
         'exact freshly built GNU 1.94 release artifact')
    return review


def prepare():
    OUT.mkdir(exist_ok=False)  # No original-owner or older profile retry.
    review = freeze()
    dest = OUT / 'input'
    dest.mkdir()
    shutil.copyfile(IMAGE, dest / 'libtest')
    shutil.copyfile(HERE / 'remote.py', dest / 'remote.py')
    shutil.copyfile(ROOT / 'scripts/ci/watchdog_linux_group.py', dest / 'group.py')
    original = ROOT / 'tools/windows-workspace-preflight-074/owned.py'
    raw = original.read_text(encoding='utf8')
    nodes = [n for n in ast.parse(raw).body if isinstance(n, ast.FunctionDef) and n.name == '_owned']
    need(len(nodes) == 1, 'one exact existing CI-envelope function')
    (dest / 'owned_function.py').write_text(ast.get_source_segment(raw, nodes[0]) + '\n',
                                          encoding='utf8', newline='\n')
    bundle = dest / 'bundle'
    bundle.mkdir()
    for name in NAMES:
        shutil.copyfile(ROOT / 'target/diagnostic-start-material-bundle-20261017' / name, bundle / name)
    # Exact selected body and transitive fixture boundaries retained for review.
    source = dest / 'source'
    source.mkdir()
    for name in ('consuming_held_live.rs', 'consuming_held_live_namespace.rs', 'consuming_held_cost.rs',
                 'held_original_service_live.rs'):
        shutil.copyfile(PARENT / 'input/fd/tools/long-run-supervisor-074/tests/support' / name, source / name)
    shutil.copyfile(original, source / 'owned-source.py')
    shutil.copyfile(PARENT / 'freeze/result.json', source / 'source-freeze.json')
    shutil.copyfile(ROOT / 'target/original-manager-live-20261028/build/native-result.json', source / 'build-receipt.json')
    for name in ('run.py', 'remote.py', 'test_controls.py', 'README.md'):
        shutil.copyfile(HERE / name, OUT / name)
    put(OUT / 'prepared.json', dict(files={p.relative_to(dest).as_posix(): sha(p)
        for p in dest.rglob('*') if p.is_file()}, protected=review['root_inputs'],
        image_sha256=IMAGE_SHA, retry=False, original_send=False, admission=False,
        product=False, windows=[300, 60, 10], external_bound=1200, retirement=5,
        cost_instrumented=True, exclusive_host=False))


def dispatch():
    need(not subprocess.check_output(['git', 'status', '--porcelain'], cwd=ROOT).strip(),
         'committed clean caller before dispatch')
    prepared = json.loads((OUT / 'prepared.json').read_bytes())
    freeze()
    dest = OUT / 'input'
    need({p.relative_to(dest).as_posix(): sha(p) for p in dest.rglob('*') if p.is_file()} == prepared['files'],
         'prepared input drift')
    need(all(sha(HERE / n) == sha(OUT / n) for n in ('run.py', 'remote.py', 'test_controls.py', 'README.md')),
         'reviewed producer drift')
    put(OUT / 'dispatch-once.json', dict(head=subprocess.check_output(['git', 'rev-parse', 'HEAD'],
        cwd=ROOT, text=True).strip(), retry=False, original_send=False, product=False))
    call('preflight', SSH + ['sudo -n /usr/bin/python3 - --preflight'],
         data=(dest / 'remote.py').read_bytes())
    before = json.loads((OUT / 'preflight.stdout.log').read_bytes())
    put(dest / 'inputs.json', dict(files=prepared['files'], expected_host=before))
    archive = OUT / 'payload.tar'
    with tarfile.open(archive, 'x') as tar:
        for path in sorted(dest.rglob('*')):
            if path.is_file():
                tar.add(path, arcname=path.relative_to(dest).as_posix(), recursive=False)
    call('upload-directory', SSH + ['umask 077; mktemp -d /tmp/hc074-preparation-upload-20261028.XXXXXX'])
    upload = (OUT / 'upload-directory.stdout.log').read_text().strip()
    need(re.fullmatch(r'/tmp/hc074-preparation-upload-20261028\.[A-Za-z0-9]{6}', upload), 'exact upload directory')
    call('upload', SCP + [str(archive), 'hydracache-ax42:' + upload + '/payload.tar'], 90)
    script = '''set -eu
test "$(id -u):$(id -g)" = 0:0
umask 077
upload=UPLOAD
test ! -L "$upload"
test "$(stat -c '%a:%u:%g' "$upload")" = 700:1000:1000
test -f "$upload/payload.tar" && test ! -L "$upload/payload.tar"
root=$(mktemp -d /tmp/hc074-preparation-root-20261028.XXXXXX)
install -m 400 "$upload/payload.tar" "$root/payload.tar"
cd "$root"
printf '%s  payload.tar\n' 'ARCHIVE_SHA' | sha256sum --check
/usr/bin/python3 -c 'import tarfile; t=tarfile.open("payload.tar"); items=t.getmembers(); assert len(items)==FILE_COUNT and all(m.isfile() and not m.name.startswith("/") and ".." not in m.name.split("/") for m in items); t.extractall(".", members=items, filter="data")'
find "$root" -type f -exec chmod 400 {} +
chmod 500 "$root/libtest"
printf '%s  remote.py\n' 'RUNNER_SHA' | sha256sum --check
printf 'ROOT=%s\n' "$root"
nohup /usr/bin/python3 -I "$root/remote.py" >"$root/live.stdout.log" 2>"$root/live.stderr.log" </dev/null &
printf 'OWNER_PID=%s\n' "$!"
'''.replace('UPLOAD', shlex.quote(upload)).replace('ARCHIVE_SHA', sha(archive)).replace(
        'RUNNER_SHA', sha(dest / 'remote.py')).replace('FILE_COUNT', str(len(prepared['files']) + 1))
    (OUT / 'stage.sh').write_text(script, encoding='ascii', newline='\n')
    call('stage', SSH + ['sudo -n /bin/sh -s'], data=script.encode())
    matches = re.findall(r'^ROOT=(/tmp/hc074-preparation-root-20261028\.[A-Za-z0-9]{6})$',
                         (OUT / 'stage.stdout.log').read_text(), re.M)
    need(len(matches) == 1, 'inspect stage; never repeat dispatch')
    put(OUT / 'remote.json', dict(root=matches[0], upload=upload, retry=False))
    print(json.dumps(dict(root=matches[0], dispatched_once=True, admission=False)))


if __name__ == '__main__':
    need(len(sys.argv) == 2 and sys.argv[1] in ('prepare', 'dispatch'), 'fixed CLI')
    (prepare if sys.argv[1] == 'prepare' else dispatch)()
