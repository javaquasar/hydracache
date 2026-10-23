"""Fixed nonexecuting bootstrap image census checks, not original admission."""
import argparse
import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('previous_bootstrap_checks', ROOT / 'tools/bootstrap-build-owner-074/check.py')
previous = importlib.util.module_from_spec(spec)
spec.loader.exec_module(previous)
logger, sha, publish = previous.logger, previous.sha, previous.publish
OUTPUT = ROOT / 'target/bootstrap-image-census-20261023'
FILES = previous.FILES + ['tools/long-run-supervisor-074/src/' + name for name in (
    'diagnostic_held_role_files.rs', 'diagnostic_held_role_census.rs', 'diagnostic_held_unit_material_kernel.rs',
    'diagnostic_bootstrap_image_census.rs', 'diagnostic_bootstrap_image_census_tests.rs',
    'diagnostic_bootstrap_image_provider.rs', 'diagnostic_bootstrap_original_inventory.rs')]
FILES += ['crates/xtask/tests/bootstrap_image_census_074.rs', 'crates/xtask/tests/support/bootstrap_image_guard.rs',
          'tools/bootstrap-build-owner-074/check.py', 'tools/bootstrap-build-owner-074/retain.py']
FILES = list(dict.fromkeys(FILES))


def commands(lane):
    steps = previous.commands(lane)
    if lane == 'host':
        steps[0][1].extend(['--test', 'bootstrap_image_census_074'])
    elif lane in ('194', '188'):
        argv = steps[0][1].copy()
        argv[argv.index('diagnostic_builder::bootstrap_build::tests')] = 'bootstrap_image_'
        steps.insert(0, ('images', argv))
    elif lane == 'metadata':
        steps[0] = ('controls', ['python', '-B', '-m', 'unittest', 'discover', '-s', 'tools/bootstrap-image-census-074', '-v'])
    else: raise ValueError('Unknown lane')
    return steps


def run(lane):
    logger.OUTPUT, logger.FILES, logger.commands = OUTPUT, FILES, commands
    OUTPUT.mkdir(exist_ok=True)
    logger.run(lane)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('lane', choices=('host', '194', '188', 'metadata'))
    run(parser.parse_args().lane)
