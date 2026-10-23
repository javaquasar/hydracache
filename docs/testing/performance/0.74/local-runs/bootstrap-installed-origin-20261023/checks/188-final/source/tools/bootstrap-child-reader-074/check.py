"""Bootstrap reader assertions/refusal only; never authenticates or starts an attempt."""
import argparse
import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('entry_checks', ROOT / 'tools/bootstrap-entry-binding-074/check.py')
previous = importlib.util.module_from_spec(spec)
spec.loader.exec_module(previous)
logger, sha, publish = previous.logger, previous.sha, previous.publish
OUTPUT = ROOT / 'target/bootstrap-child-reader-20261023'
PACKAGE = 'hydracache-long-run-supervisor-074'
FILES = previous.FILES + ['tools/long-run-supervisor-074/' + name for name in (
    'src/bin/diagnostic_held_bootstrap.rs', 'src/bin/diagnostic_held_launcher.rs',
    'src/diagnostic_bootstrap_child_ingress.rs', 'src/diagnostic_bootstrap_child_reader.rs',
    'src/diagnostic_bootstrap_child_reader_tests.rs', 'tests/bootstrap_child_reader.rs',
    'tests/support/bootstrap_child_material.rs', 'Cargo.toml')]
FILES += ['crates/xtask/tests/bootstrap_child_reader_074.rs', 'crates/xtask/tests/support/bootstrap_child_guard.rs']
FILES += ['tools/bootstrap-child-reader-074/' + name for name in ('check.py', 'retain.py', 'test_controls.py', 'README.md')]
FILES = list(dict.fromkeys(FILES))


def commands(lane):
    steps = previous.commands(lane)
    if lane == 'metadata':
        steps[0] = ('controls', ['python', '-B', '-m', 'unittest', 'discover', '-s', 'tools/bootstrap-child-reader-074', '-v'])
        return steps
    if lane == 'host':
        steps[0][1].extend(['--test', 'bootstrap_child_reader_074'])
        base = ['cargo']
    elif lane in ('194', '188'):
        argv = steps[0][1]
        base = argv[:argv.index('test')]
    else: raise ValueError('Unknown lane')
    common = ['test', '--offline', '--locked'] + (['--release'] if lane != 'host' else []) + ['-p', PACKAGE]
    steps[:0] = [('child-units', base + common + ['--bin', 'diagnostic_held_bootstrap', '--', '--nocapture']),
                 ('child-cli', base + common + ['--test', 'bootstrap_child_reader', '--', '--nocapture'])]
    if lane == '194':
        # Exact selected command after cargo/+toolchain; offline is an environment
        # constraint, not an altered receipt assertion or qualification workload.
        build = base.copy()
        build.insert(build.index('/home/artur/.cargo/bin/cargo'), 'CARGO_NET_OFFLINE=true')
        steps.append(('selected-build', build + ['build', '--manifest-path', 'tools/long-run-supervisor-074/Cargo.toml',
            '--release', '--locked', '--no-default-features', '--bin', 'diagnostic_held_bootstrap',
            '--target', 'x86_64-unknown-linux-gnu', '--message-format=json']))
        image = '/home/artur/.cache/hydracache-074-194-check/x86_64-unknown-linux-gnu/release/diagnostic_held_bootstrap'
        steps.append(('image-sha', ['wsl.exe', '-d', 'Ubuntu', '-u', 'artur', '--', '/usr/bin/sha256sum', image]))
        steps.append(('image-size', ['wsl.exe', '-d', 'Ubuntu', '-u', 'artur', '--', '/usr/bin/stat', '-c', '%s', image]))
    return steps


def run(lane):
    logger.OUTPUT, logger.FILES, logger.commands = OUTPUT, FILES, commands
    OUTPUT.mkdir(exist_ok=True)
    logger.run(lane)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('lane', choices=('host', '194', '188', 'metadata'))
    run(parser.parse_args().lane)
