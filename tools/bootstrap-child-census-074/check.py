"""Fixed local complete-role assertion controls; original authentication stays closed."""
import argparse
import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('child_reader_checks', ROOT / 'tools/bootstrap-child-reader-074/check.py')
previous = importlib.util.module_from_spec(spec)
spec.loader.exec_module(previous)
logger, sha, publish = previous.logger, previous.sha, previous.publish
OUTPUT = ROOT / 'target/bootstrap-child-census-20261023'
FILES = previous.FILES + ['tools/long-run-supervisor-074/' + name for name in (
    'src/diagnostic_bootstrap_census_entry.rs', 'src/diagnostic_bootstrap_child_roles.rs',
    'src/diagnostic_bootstrap_child_role_kernel.rs', 'src/diagnostic_bootstrap_closed_inventory.rs',
    'src/diagnostic_bootstrap_child_roles_tests.rs', 'tests/bootstrap_child_census.rs',
    'tests/support/bootstrap_census_material.rs')]
FILES += ['crates/xtask/tests/bootstrap_child_census_074.rs', 'crates/xtask/tests/support/bootstrap_roles_guard.rs']
FILES += ['tools/bootstrap-child-census-074/' + name for name in ('check.py', 'retain.py', 'test_controls.py', 'capture_initial.py', 'README.md')]
FILES += ['tools/bootstrap-child-reader-074/check.py', 'tools/bootstrap-child-reader-074/retain.py']
FILES = list(dict.fromkeys(FILES))


def commands(lane):
    lane = lane.removesuffix('-final')
    steps = previous.commands(lane)
    if lane == 'metadata':
        steps[0] = ('controls', ['python', '-B', '-m', 'unittest', 'discover', '-s', 'tools/bootstrap-child-census-074', '-v'])
    elif lane == 'host':
        next(argv for label, argv in steps if label == 'canaries').extend(['--test', 'bootstrap_child_census_074'])
    elif lane in ('194', '188'):
        argv = next(argv for label, argv in steps if label == 'child-cli').copy()
        argv[argv.index('bootstrap_child_reader')] = 'bootstrap_child_census'
        steps.insert(0, ('census-cli', argv))
    else: raise ValueError('Unknown lane')
    return steps


def run(lane):
    logger.OUTPUT, logger.FILES, logger.commands = OUTPUT, FILES, commands
    OUTPUT.mkdir(exist_ok=True)
    logger.run(lane)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('lane', choices=('host', '194', '188', 'metadata', 'host-final', '194-final', '188-final', 'metadata-final'))
    run(parser.parse_args().lane)
