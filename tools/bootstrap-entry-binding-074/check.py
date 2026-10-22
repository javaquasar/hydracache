"""Fixed nonexecuting entry codec/private-FD/model checks; never original start."""
import argparse
import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('image_component_checks', ROOT / 'tools/bootstrap-image-census-074/check.py')
previous = importlib.util.module_from_spec(spec)
spec.loader.exec_module(previous)
logger, sha, publish = previous.logger, previous.sha, previous.publish
OUTPUT = ROOT / 'target/bootstrap-entry-binding-20261023'
FILES = previous.FILES + ['tools/long-run-supervisor-074/src/' + name for name in (
    'diagnostic_bootstrap_entry_format.rs', 'diagnostic_bootstrap_entry_binding.rs',
    'diagnostic_bootstrap_entry_binding_tests.rs', 'diagnostic_bootstrap_entry_resource.rs',
    'diagnostic_bootstrap_entry_census.rs')]
FILES += ['crates/xtask/tests/bootstrap_entry_binding_074.rs', 'crates/xtask/tests/support/bootstrap_entry_guard.rs',
          'tools/bootstrap-image-census-074/check.py', 'tools/bootstrap-image-census-074/retain.py',
          'crates/xtask/tests/support/bootstrap_enrollment_guard.rs']
FILES += ['tools/bootstrap-entry-binding-074/' + name for name in ('check.py', 'retain.py', 'test_controls.py', 'README.md')]
FILES = list(dict.fromkeys(FILES))


def commands(lane):
    steps = previous.commands(lane)
    if lane == 'host':
        steps[0][1].extend(['--test', 'bootstrap_entry_binding_074'])
        argv = steps[1][1].copy()
        argv[argv.index('diagnostic_builder::bootstrap_build::tests')] = 'bootstrap_entry_format'
        steps.insert(1, ('entry', argv))
    elif lane in ('194', '188'):
        argv = steps[0][1].copy()
        argv[argv.index('bootstrap_image_')] = 'bootstrap_entry_'
        steps.insert(0, ('entry', argv))
    elif lane == 'metadata':
        steps[0] = ('controls', ['python', '-B', '-m', 'unittest', 'discover', '-s', 'tools/bootstrap-entry-binding-074', '-v'])
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
