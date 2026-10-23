"""Fixed local current-image consistency controls, never original authentication."""
import argparse
import importlib.util
import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('census_component_checks', ROOT / 'tools/bootstrap-child-census-074/check.py')
previous = importlib.util.module_from_spec(spec)
spec.loader.exec_module(previous)
logger, sha, publish = previous.logger, previous.sha, previous.publish
OUTPUT = ROOT / 'target/bootstrap-current-image-20261023'
FILES = previous.FILES + ['tools/long-run-supervisor-074/' + n for n in (
    'src/diagnostic_bootstrap_current_image.rs', 'src/diagnostic_bootstrap_image_entry.rs',
    'tests/bootstrap_current_image.rs')]
FILES += ['crates/xtask/tests/bootstrap_current_image_074.rs', 'crates/xtask/tests/support/bootstrap_current_image_guard.rs']
FILES += ['tools/bootstrap-current-image-074/' + n for n in ('check.py', 'retain.py', 'test_controls.py', 'README.md')]
FILES = list(dict.fromkeys(FILES))
LANES = ('host', '194', '188', 'metadata', 'host-final', '194-final', '188-final', 'metadata-final')


def commands(lane):
    if lane not in LANES: raise ValueError('Unknown lane')
    kind = lane.removesuffix('-final')
    steps = previous.commands(kind)
    if kind == 'host':
        next(argv for name, argv in steps if name == 'canaries').extend(['--test', 'bootstrap_current_image_074'])
    elif kind in ('194', '188'):
        argv = next(argv for name, argv in steps if name == 'census-cli').copy()
        argv[argv.index('bootstrap_child_census')] = 'bootstrap_current_image'
        steps.insert(0, ('image-cli', argv))
    else:
        steps[0] = ('controls', ['python', '-B', '-m', 'unittest', 'discover', '-s', 'tools/bootstrap-current-image-074', '-v'])
    return steps


def run(lane):
    steps = commands(lane)
    folder = OUTPUT / lane
    folder.mkdir(parents=True)
    sources = {n: sha(ROOT / n) for n in FILES}
    publish(folder / 'source-before.json', sources)
    for name in FILES:
        dest = folder / 'source' / name
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(ROOT / name, dest)
        if sha(dest) != sources[name]: raise ValueError('Initial source drift')
    result = dict(lane=lane, failure=None, steps=[], original_attempt=False, server=False, product=False)
    try:
        for name, argv in steps:
            start = logger.time.monotonic_ns()
            try:
                child = logger.subprocess.run(argv, capture_output=True, cwd=ROOT, timeout=1800)
                stdout, stderr, code = child.stdout, child.stderr, child.returncode
            except logger.subprocess.TimeoutExpired as error:
                stdout, stderr, code = error.stdout or b'', error.stderr or b'', None
            for suffix, raw in (('stdout', stdout), ('stderr', stderr)):
                with (folder / (name + '.' + suffix + '.log')).open('xb') as stream: stream.write(raw)
            publish(folder / (name + '.json'), dict(argv=argv, exit=code,
                elapsed_ns=logger.time.monotonic_ns() - start,
                stdout_sha256=logger.hashlib.sha256(stdout).hexdigest(), stderr_sha256=logger.hashlib.sha256(stderr).hexdigest()))
            result['steps'].append(name)
            print(lane, name, code, flush=True)
            if code != 0: raise RuntimeError(name + ' refused; first result retained')
        after = {n: sha(ROOT / n) for n in FILES}
        publish(folder / 'source-after.json', after)
        if after != sources: raise RuntimeError('Source drift')
    except Exception as error:
        result['failure'] = type(error).__name__ + ': ' + str(error)
    publish(folder / 'result.json', result)
    if result['failure']: raise RuntimeError(result['failure'])


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('lane', choices=LANES)
    run(parser.parse_args().lane)
