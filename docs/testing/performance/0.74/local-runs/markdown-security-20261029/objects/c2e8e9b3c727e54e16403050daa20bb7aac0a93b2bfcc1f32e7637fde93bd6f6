"""One approved local trace lane; exclusive intent, no redispatch or retry."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent
OUT = ROOT / 'target/wsl-clock-trace-20261023'
ARCHIVE = ROOT / 'docs/testing/performance/0.74/local-runs/wsl-clock-trace-20261023'
PLAN = ROOT / 'docs/testing/performance/0.74/wsl-clock-trace-plan-20261023.md'
FROZEN = {
    'qualification-manifest.toml': '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc',
    'local-runs/bootstrap-original-getunit-20261023/index.json': '9f2fd0f457ec6d48f4e87e7bfc2bd1262bbf191cf51b760fe744eb369394b9f6',
    'local-runs/wsl-clock-readonly-20261023/index.json': 'edec06e2ac4fa97126ce14ac5c0e4d1aff9cae7bd39f96bb91391102f3d606e3',
    'local-runs/wsl-clock-causality-20261023/index.json': '6a7d11e8e8dc830262b8191bacdce03b1a0b822261d246dbaae0991da0694298',
    'local-runs/wsl-clock-causality-root-context-20261023/index.json': '798b204c4b14865f482d64cdc4176aec7c1c3c4126b87bf635e8d9c2ba140082',
    'local-runs/wsl-clock-offline-join-20261023/index.json': '56250b03ab2aab0efb3cd1d5e559fcae6f5e0e64314f75693297d7ef4bff8aff',
}


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def put(path, value):
    with path.open('x', encoding='utf-8', newline='\n') as stream:
        json.dump(value, stream, indent=2, sort_keys=True)
        stream.write('\n')
        stream.flush()
        os.fsync(stream.fileno())


def sources():
    for name, expected in FROZEN.items():
        path = ROOT / 'docs/testing/performance/0.74' / name
        if digest(path.read_bytes()) != expected:
            raise ValueError('Frozen input changed: ' + name)
    subprocess.run(['git', 'diff', '--exit-code', '8fb0fb5a', '--',
                    'tools/long-run-supervisor-074/src', 'tools/long-run-supervisor-074/tests',
                    'Cargo.lock', 'docs/COMPAT.md'], cwd=ROOT, capture_output=True, check=True)
    return {p.relative_to(ROOT).as_posix(): digest(p.read_bytes())
            for p in sorted(HERE.iterdir()) if p.is_file()} | {PLAN.relative_to(ROOT).as_posix(): digest(PLAN.read_bytes())}


def linux(path):
    value = path.resolve().as_posix()
    if not value.startswith('C:/Workspace/prj/jq/cashe/hydracache-074/'):
        raise ValueError('Not the exact Windows workspace')
    return '/mnt/c/' + value[3:]


def command(name, argv, timeout):
    with (OUT / (name + '.stdout.log')).open('xb') as out, (OUT / (name + '.stderr.log')).open('xb') as err:
        try:
            rc = subprocess.run(argv, cwd=ROOT, stdout=out, stderr=err, timeout=timeout, check=False).returncode
        except subprocess.TimeoutExpired:
            rc = 124  # Never redispatch. The independently armed guardian has its own RAW deadline.
    put(OUT / (name + '.json'), {'argv': argv, 'exit': rc, 'timeout_seconds': timeout})
    if rc:
        raise RuntimeError(f'{name}: first exit {rc}; no retry')


def collect():
    if subprocess.check_output(['git', 'status', '--porcelain'], cwd=ROOT).strip():
        raise ValueError('Commit the reviewed collector/plan before collection')
    proof = sources()
    OUT.mkdir(parents=True, exist_ok=False)
    put(OUT / 'intent.json', {'schema': 'hc074-bounded-clock-trace-intent-v1',
        'sources': proof, 'frozen_inputs': FROZEN,
        'git': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
        'authorization': 'explicit user approval of one bounded private tracefs instance and temporary clock-path probes',
        'tracing_state_changed': True, 'clock_change': False, 'service_change': False,
        'original_owner_run': False, 'server': False, 'product': False, 'admission': False,
        'original_windows': [300, 60, 10], 'retry': False})
    command('controls', ['wsl.exe', '-d', 'Ubuntu', '-u', 'artur', '--exec',
        'python3', '-B', '-m', 'unittest', 'discover', '-s', linux(HERE), '-v'], 45)
    command('session', ['wsl.exe', '-d', 'Ubuntu', '-u', 'root', '--exec',
        'python3', '-B', linux(HERE / 'capture.py')], 180)
    if sources() != proof:
        raise ValueError('Collector or frozen inputs drifted')
    # Analysis may refuse; raw evidence/first outcome is retained regardless.
    import analyze
    put(OUT / 'analysis.json', analyze.analyze(OUT))


def retain():
    proof = sources()
    intent = json.loads((OUT / 'intent.json').read_text())
    if intent['sources'] != proof or intent['frozen_inputs'] != FROZEN:
        raise ValueError('Cannot seal different sources or input pins')
    ARCHIVE.mkdir(parents=True, exist_ok=False)
    objects = ARCHIVE / 'objects'
    objects.mkdir()
    files = {}
    inputs = [('observation/' + p.name, p.read_bytes()) for p in sorted(OUT.iterdir()) if p.is_file()]
    inputs += [('source/' + name, (ROOT / name).read_bytes()) for name in proof]
    for name, raw in inputs:
        sha = digest(raw)
        if not (objects / sha).exists():
            with (objects / sha).open('xb') as stream:
                stream.write(raw)
        files[name] = {'sha256': sha, 'bytes': len(raw)}
    put(ARCHIVE / 'index.json', {'schema': 'hc074-bounded-clock-trace-evidence-v1',
        'files': files, 'sources': proof, 'frozen_inputs': FROZEN,
        'tracing_state_changed': True, 'clock_change': False, 'service_change': False,
        'original_owner_run': False, 'server': False, 'product': False, 'admission': False,
        'original_windows': [300, 60, 10]})
    verify()


def verify():
    index = json.loads((ARCHIVE / 'index.json').read_text())
    if index['schema'] != 'hc074-bounded-clock-trace-evidence-v1' or index['frozen_inputs'] != FROZEN:
        raise ValueError('Unexpected evidence schema or frozen inputs')
    if index['original_windows'] != [300, 60, 10] or index['tracing_state_changed'] is not True:
        raise ValueError('Incorrect scope')
    if any(index[k] is not False for k in ('clock_change', 'service_change', 'original_owner_run', 'server', 'product', 'admission')):
        raise ValueError('Not the authorized non-admitting scope')
    for entry in index['files'].values():
        sha = entry['sha256']
        if len(sha) != 64 or any(c not in '0123456789abcdef' for c in sha):
            raise ValueError('Unsafe object hash')
        raw = (ARCHIVE / 'objects' / sha).read_bytes()
        if len(raw) != entry['bytes'] or digest(raw) != sha:
            raise ValueError('Changed evidence bytes')
    print(json.dumps({'files': len(index['files']), 'index_sha256': digest((ARCHIVE / 'index.json').read_bytes())}))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=('collect', 'retain', 'verify'))
    {'collect': collect, 'retain': retain, 'verify': verify}[parser.parse_args().mode]()
