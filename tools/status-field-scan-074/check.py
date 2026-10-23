"""Local compile/lint/pure and filesystem controls; no original-owner admission."""
from pathlib import Path
import hashlib
import json
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'target/status-field-scan-20261024'


def snapshot():
    paths = [ROOT / 'Cargo.lock', ROOT / 'Cargo.toml']
    for folder in ('tools/long-run-supervisor-074/src', 'tools/long-run-supervisor-074/tests', 'tools/status-field-scan-074'):
        paths += [path for path in (ROOT / folder).rglob('*') if path.is_file() and '__pycache__' not in path.parts]
    return {path.relative_to(ROOT).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest() for path in sorted(paths)}


def run(lane):
    if lane not in ('194', '188', '194-pathfix', '188-pathfix', '194-source-view', '188-source-view', '194-final', '188-final'):
        raise ValueError('Compiler lane')
    compiler = lane.split('-', 1)[0]
    folder = OUT / ('checks-' + lane)
    folder.mkdir()
    before = snapshot()
    (folder / 'source-before.json').write_text(json.dumps(before, indent=2), encoding='utf-8')
    version = '1.94.0' if compiler == '194' else '1.88.0'
    prefix = ['wsl.exe', '-d', 'Ubuntu', '-u', 'artur', '--exec', '/usr/bin/env', 'RUSTUP_AUTO_INSTALL=0',
        'CARGO_INCREMENTAL=0', f'CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-{compiler}-check',
        '/home/artur/.cargo/bin/cargo', '+' + version]
    package = ['--offline', '--locked', '-p', 'hydracache-long-run-supervisor-074']
    commands = [('check', prefix + ['check'] + package + ['--all-targets']),
        ('clippy', prefix + ['clippy'] + package + ['--all-targets', '--all-features', '--', '-D', 'warnings'])]
    commands += [(selector.replace(':', '_'), prefix + ['test'] + package + ['--release', '--all-features', '--lib', selector,
                  '--', '--test-threads=1']) for selector in
                 ('status_field_scan_', 'diagnostic_kernel_admission::', 'projection_', 'fresh_status_', 'fixed_leaf_path_', 'authority_')]
    steps = []
    failure = None
    try:
        for name, argv in commands:
            if snapshot() != before:
                raise ValueError('Source changed during checks')
            with (folder / (name + '-intent.json')).open('x') as stream:
                json.dump({'argv': argv, 'server': False, 'original_owner_run': False}, stream, indent=2)
            with (folder / (name + '.stdout.log')).open('xb') as stdout, (folder / (name + '.stderr.log')).open('xb') as stderr:
                result = subprocess.run(argv, cwd=ROOT, stdout=stdout, stderr=stderr, timeout=1200, check=False)
            steps.append(dict(name=name, exit=result.returncode))
            print(steps[-1], flush=True)
            if result.returncode:
                raise ValueError('First local check failure retained')
    except Exception as error:
        failure = str(error)
    after = snapshot()
    with (folder / 'source-after.json').open('x') as stream:
        json.dump(after, stream, indent=2)
    with (folder / 'result.json').open('x') as stream:
        json.dump(dict(lane=lane, steps=steps, failure=failure, source_unchanged=after == before,
                       server=False, product=False, original_owner_run=False), stream, indent=2)
    if failure or after != before:
        raise SystemExit(1)


if __name__ == '__main__':
    run(sys.argv[1])
