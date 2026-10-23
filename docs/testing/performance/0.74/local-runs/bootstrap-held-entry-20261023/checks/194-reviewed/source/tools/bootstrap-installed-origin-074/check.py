"""Local installed-origin source/filesystem/models; no installation or original child."""
import argparse
import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('installed_snapshot_runner', ROOT / 'tools/bootstrap-current-image-074/check.py')
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)
logger, sha, publish = runner.logger, runner.sha, runner.publish
OUTPUT = ROOT / 'target/bootstrap-installed-origin-20261023'
LANES = runner.LANES
FILES = runner.FILES + ['tools/long-run-supervisor-074/src/' + n for n in (
    'diagnostic_bootstrap_installed_source.rs', 'diagnostic_bootstrap_installed_source_tests.rs',
    'diagnostic_bootstrap_installed_owner.rs', 'diagnostic_bootstrap_installed_owner_tests.rs',
    'diagnostic_bootstrap_installed_census.rs', 'diagnostic_bootstrap_installed_census_tests.rs')]
FILES += ['crates/xtask/tests/bootstrap_installed_origin_074.rs', 'crates/xtask/tests/bootstrap_child_boundary_074.rs',
          'crates/xtask/tests/support/bootstrap_installed_guard.rs']
FILES += ['tools/bootstrap-installed-origin-074/' + n for n in ('check.py', 'retain.py', 'test_controls.py', 'README.md')]
FILES = list(dict.fromkeys(FILES))


def commands(lane):
    if lane not in LANES: raise ValueError('Unknown lane')
    kind = lane.removesuffix('-final')
    steps = runner.commands(lane)
    if kind == 'host':
        next(argv for name, argv in steps if name == 'canaries').extend([
            '--test', 'bootstrap_child_boundary_074', '--test', 'bootstrap_installed_origin_074'])
    elif kind in ('194', '188'):
        argv = next(argv for name, argv in steps if name == 'entry').copy()
        argv[argv.index('bootstrap_entry_')] = 'bootstrap_installed_'
        steps.insert(0, ('installed', argv))
    else:
        steps[0] = ('controls', ['python', '-B', '-m', 'unittest', 'discover', '-s', 'tools/bootstrap-installed-origin-074', '-v'])
    return steps


def run(lane):
    # Keep a private command factory: replacing runner.commands would recurse.
    saved = runner.commands
    steps = commands(lane)
    runner.OUTPUT, runner.FILES = OUTPUT, FILES
    runner.commands = lambda actual: steps if actual == lane else (_ for _ in ()).throw(ValueError('Wrong lane'))
    try: runner.run(lane)
    finally: runner.commands = saved


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('lane', choices=LANES)
    run(parser.parse_args().lane)
