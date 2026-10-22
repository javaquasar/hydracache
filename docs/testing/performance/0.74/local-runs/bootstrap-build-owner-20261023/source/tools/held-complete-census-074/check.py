"""Closed role census component checks; never starts an original attempt or unit."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]
OUTPUT = ROOT / 'target/held-complete-census-20261022'
PACKAGE = 'hydracache-long-run-supervisor-074'
FILES = ['tools/long-run-supervisor-074/src/' + name for name in (
    'diagnostic_artifacts_linux.rs', 'diagnostic_bundle_projection.rs',
    'diagnostic_bundle_role_census.rs', 'diagnostic_root_guardian_projection.rs',
    'diagnostic_guardian_role_census.rs', 'diagnostic_held_role_census.rs',
    'diagnostic_held_role_census_tests.rs', 'diagnostic_material_role_census.rs',
    'diagnostic_held_unit_material.rs', 'diagnostic_held_unit_material_kernel.rs',
    'diagnostic_held_unit_material_tests.rs', 'diagnostic_held_unit_material_kernel_tests.rs',
    'diagnostic_consuming_held_join.rs', 'diagnostic_held_unit_projection.rs',
    'diagnostic_manager_channel.rs', 'diagnostic_original_manager_message.rs',
    'diagnostic_original_watchdog_context.rs', 'diagnostic_root_guardian.rs',
    'diagnostic_root_guardian_raw.rs', 'diagnostic_root_guardian_original_kernel.rs',
    'diagnostic_held_launcher_kernel.rs')]
FILES += ['crates/xtask/tests/' + name for name in (
    'held_complete_role_census_074.rs', 'held_material_roles_074.rs',
    'support/held_role_census_guard.rs', 'held_material_074.rs', 'held_unit_projection_074.rs')]
FILES += ['Cargo.lock', 'docs/testing/performance/0.74/qualification-manifest.toml']


def sha(file): return hashlib.sha256(file.read_bytes()).hexdigest()


def publish(file, data):
    with file.open('x', encoding='utf-8') as stream:
        json.dump(data, stream, sort_keys=True, indent=2)


def commands(lane):
    if lane in ('controls', 'controls-v2'):
        return [('controls', ['python', '-B', '-m', 'unittest', 'discover', '-s',
                              'tools/held-complete-census-074', '-v'])]
    if lane in ('docs', 'docs-v2'):
        return [('docs', ['cargo', 'xtask', 'doc-check']),
                ('registry', ['cargo', 'xtask',
                              'gated-test-check' if lane == 'docs-v2' else 'gated-test-registry-check']),
                ('contract', ['cargo', 'xtask', 'performance-contract-check', '--release', '0.74']),
                ('diff', ['git', 'diff', '--check'])]
    if lane == 'host':
        return [('canaries', ['cargo', 'test', '--offline', '--locked', '-p', 'xtask'] +
                 [arg for test in ('held_complete_role_census_074', 'held_material_roles_074',
                                  'held_material_074', 'held_unit_projection_074') for arg in ('--test', test)]),
                ('check', ['cargo', 'check', '--offline', '--locked', '-p', PACKAGE, '-p', 'xtask', '--all-targets']),
                ('clippy', ['cargo', 'clippy', '--offline', '--locked', '-p', PACKAGE, '-p', 'xtask',
                           '--all-targets', '--all-features', '--', '-D', 'warnings']),
                ('format', ['cargo', 'fmt', '-p', PACKAGE, '-p', 'xtask', '--', '--check'])]
    if lane not in ('194', '188'): raise ValueError('Unknown lane')
    base = ['wsl.exe', '-d', 'Ubuntu', '-u', 'artur', '--', 'env', 'RUSTUP_AUTO_INSTALL=0',
            'CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-' + lane + '-check',
            '/home/artur/.cargo/bin/cargo', '+' + ('1.94.0' if lane == '194' else '1.88.0')]
    tests = [(label, base + ['test', '--offline', '--locked', '--release', '-p', PACKAGE,
                            '--all-features', '--lib', pattern, '--', '--nocapture', '--test-threads=1'])
             for label, pattern in (('census', 'held_complete_census'), ('material', 'held_material'),
                                    ('projection', 'held_unit_projection'))]
    return tests + [('check', base + ['check', '--offline', '--locked', '-p', PACKAGE, '--all-targets']),
                    ('clippy', base + ['clippy', '--offline', '--locked', '-p', PACKAGE,
                                      '--all-targets', '--all-features', '--', '-D', 'warnings'])]


def run(lane):
    steps = commands(lane)
    folder = OUTPUT / lane
    folder.mkdir()
    sources = {name: sha(ROOT / name) for name in FILES}
    publish(folder / 'source-before.json', sources)
    result = dict(lane=lane, failure=None, steps=[], original_attempt=False, server=False, product=False)
    try:
        for name, argv in steps:
            start = time.monotonic_ns()
            try:
                child = subprocess.run(argv, capture_output=True, cwd=ROOT, timeout=1800)
                stdout, stderr, code = child.stdout, child.stderr, child.returncode
            except subprocess.TimeoutExpired as error:
                stdout, stderr, code = error.stdout or b'', error.stderr or b'', None
            for suffix, raw in (('stdout', stdout), ('stderr', stderr)):
                with (folder / (name + '.' + suffix + '.log')).open('xb') as stream: stream.write(raw)
            publish(folder / (name + '.json'), dict(argv=argv, exit=code,
                elapsed_ns=time.monotonic_ns() - start, stdout_sha256=hashlib.sha256(stdout).hexdigest(),
                stderr_sha256=hashlib.sha256(stderr).hexdigest()))
            result['steps'].append(name)
            print(lane, name, code, flush=True)
            if code != 0: raise RuntimeError(name + ' refused; first result retained')
        after = {name: sha(ROOT / name) for name in FILES}
        publish(folder / 'source-after.json', after)
        if after != sources: raise RuntimeError('Source drift')
    except Exception as error: result['failure'] = type(error).__name__ + ': ' + str(error)
    publish(folder / 'result.json', result)
    if result['failure']: raise RuntimeError(result['failure'])


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('lane', choices=('host', '194', '188', 'controls', 'docs', 'controls-v2', 'docs-v2'))
    run(parser.parse_args().lane)
