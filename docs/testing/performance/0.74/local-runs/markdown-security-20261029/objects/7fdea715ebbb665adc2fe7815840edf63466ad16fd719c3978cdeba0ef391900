"""Focused source/model/kernel checks only; no original attempt or unit start."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]
OUTPUT = ROOT / 'target/held-material-roles-20261022'
FILES = ['tools/long-run-supervisor-074/src/' + name for name in (
    'diagnostic_held_unit_material.rs', 'diagnostic_held_unit_material_kernel.rs',
    'diagnostic_held_unit_material_tests.rs', 'diagnostic_held_unit_material_kernel_tests.rs')]
FILES += ['crates/xtask/tests/held_material_roles_074.rs', 'Cargo.lock',
          'docs/testing/performance/0.74/qualification-manifest.toml']


def sha(file): return hashlib.sha256(file.read_bytes()).hexdigest()


def publish(file, data):
    with file.open('x') as output: json.dump(data, output, sort_keys=True, indent=2)


def run(lane):
    output = OUTPUT / lane
    output.mkdir()
    sources = {name: sha(ROOT / name) for name in FILES}
    publish(output / 'source-before.json', sources)
    package = 'hydracache-long-run-supervisor-074'
    if lane == 'host':
        commands = [('canaries', ['cargo', 'test', '--offline', '--locked', '-p', 'xtask', '--test',
            'held_material_roles_074', '--test', 'held_material_074', '--test', 'held_unit_projection_074']),
            ('check', ['cargo', 'check', '--offline', '--locked', '-p', package, '-p', 'xtask', '--all-targets']),
            ('clippy', ['cargo', 'clippy', '--offline', '--locked', '-p', package, '-p', 'xtask',
                '--all-targets', '--all-features', '--', '-D', 'warnings']),
            ('format', ['cargo', 'fmt', '-p', package, '-p', 'xtask', '--', '--check'])]
    else:
        compiler = '1.94.0' if lane == '194' else '1.88.0'
        base = ['wsl.exe', '-d', 'Ubuntu', '-u', 'artur', '--', 'env', 'RUSTUP_AUTO_INSTALL=0',
            'CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-' + lane + '-check',
            '/home/artur/.cargo/bin/cargo', '+' + compiler]
        commands = [('material', base + ['test', '--offline', '--locked', '--release', '-p', package,
            '--all-features', '--lib', 'held_material', '--', '--nocapture', '--test-threads=1']),
            ('projection', base + ['test', '--offline', '--locked', '--release', '-p', package,
            '--all-features', '--lib', 'held_unit_projection', '--', '--nocapture', '--test-threads=1']),
            ('check', base + ['check', '--offline', '--locked', '-p', package, '--all-targets']),
            ('clippy', base + ['clippy', '--offline', '--locked', '-p', package, '--all-targets',
                '--all-features', '--', '-D', 'warnings'])]
    result = dict(lane=lane, failure=None, steps=[], original_attempt=False, server=False, product=False)
    try:
        for name, argv in commands:
            start = time.monotonic_ns()
            try:
                child = subprocess.run(argv, capture_output=True, cwd=ROOT, timeout=1800)
                stdout, stderr, code = child.stdout, child.stderr, child.returncode
            except subprocess.TimeoutExpired as error:
                stdout, stderr, code = error.stdout or b'', error.stderr or b'', None
            for suffix, raw in (('stdout', stdout), ('stderr', stderr)):
                with (output / (name + '.' + suffix + '.log')).open('xb') as stream: stream.write(raw)
            row = dict(argv=argv, exit=code, elapsed_ns=time.monotonic_ns() - start,
                stdout_sha256=hashlib.sha256(stdout).hexdigest(), stderr_sha256=hashlib.sha256(stderr).hexdigest())
            publish(output / (name + '.json'), row)
            result['steps'].append(name)
            print(lane, name, code, flush=True)
            if code != 0: raise RuntimeError(name + ' refused; inspect retained first outcome')
        after = {name: sha(ROOT / name) for name in FILES}
        publish(output / 'source-after.json', after)
        if sources != after: raise RuntimeError('Source changed during checks')
    except Exception as error: result['failure'] = type(error).__name__ + ': ' + str(error)
    publish(output / 'result.json', result)
    if result['failure']: raise RuntimeError(result['failure'])


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('lane', choices=('host', '194', '188'))
    run(parser.parse_args().lane)
